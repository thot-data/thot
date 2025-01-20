#' Create a new Syre database connection.
#'
#' @param dev_root Interactive graph root.
#'  Used to set the development root when using a script interactively.
#'  The script will set its graph root to the `Container` at the given path.
#'  This is ignored if the script is being run by a runner.
#' @param chdir Change the working directory to the analysis root.
#'  Defaults to `True`.
#'
#' @returns A Syre Database connection.
#' @export
#'
#' @examples
#' db <- database(dev_root = "/path/to/my/syre/project/container")
database <- function(dev_root = NULL, chdir = TRUE) {
  if (!database_available()) {
    exe_path <- database_server_path()
    system(exe_path, wait = FALSE)
  }

  socket <- zmq_socket()
  project_id <- syre_project_id()
  root_path <- syre_container_path()
  if (is.na(project_id) && is.na(root_path)) {
    if (is.null(dev_root)) {
      stop("`dev_root` must be set")
    }
    database_dev(socket, dev_root, chdir)
  } else if (!is.na(project_id) && !is.na(root_path)) {
    database_prod(socket, project_id, root_path, chdir)
  } else {
    stop(sprintf(
      "`%s` and `%s` must both be either set or not set",
      PROJECT_ID_KEY,
      CONTAINER_ID_KEY
    ))
  }
}

#' Initialize a database in a development environment.
#'
#' @param socket ZMQ socket to use.
#' @param dev_root Absolute system path to root container for database.
#' @param chdir Change directory to project analyses folder.
database_dev <- function(socket, dev_root, chdir) {
  if (!isAbsolutePath(dev_root)) {
    stop("`dev_root` must be an absolute path")
  }

  root_path <- normalizePath(dev_root, mustWork = TRUE)
  if (!file.exists(root_path)) {
    stop("Root path does not exist")
  }

  project_manifest <- send_cmd(socket, '{"State": "ProjectManifest"}')

  root_path_parts <- split_path(root_path)
  project_path <- NA
  for (path in project_manifest) {
    path_parts <- tryCatch(
      {
        split_path(normalizePath(path, mustWork = TRUE))
      },
      error = function(cond) {
        NA
      }
    )
    if (any(is.na(path_parts))) {
      next
    }

    if (length(path_parts) > length(root_path_parts)) {
      next
    }

    path_match <- TRUE
    for (idx in seq_along(path_parts)) {
      if (path_parts[idx] != root_path_parts[idx]) {
        path_match <- FALSE
        break
      }
    }

    if (path_match) {
      project_path <- path
      break
    }
  }

  if (is.na(project_path)) {
    stop("Path is not in a project")
  }

  cmd <- sprintf('{"Project": {"Get": "%s"}}', escape_str(project_path))
  project <- send_cmd(socket, cmd, result = FALSE)
  if (is.null(project)) {
    stop("Could not get project")
  }

  stopifnot(project$path == project_path)
  project <- project$fs_resource$Present
  if (is.null(project)) {
    stop("Project folder is missing")
  }

  project_properties <- project$properties$Ok
  if (is.null(project_properties)) {
    stop("Project properties are invalid")
  }

  project_id <- project_properties$rid
  data_root <- normalizePath(
    file.path(project_path, project_properties$data_root),
    mustWork = TRUE
  )
  root <- relpath(root_path, data_root)
  root <- paste(.Platform$file.sep, root, sep = "")

  root_container <- get_container(socket, project_id, root)
  if (is.null(root_container)) {
    stop("Could not get root container")
  }

  root_properties <- root_container$properties$Ok
  if (is.null(root_properties)) {
    stop(
      "Root container properties file is corrupt:",
      root_container$properties$Err
    )
  }

  root_id <- root_properties$rid

  creator <- active_user(socket)
  creator <- list(User = list(Id = creator))

  if (chdir) {
    analysis_root <- project_properties$analysis_root
    if (is.null(analysis_root)) {
      stop("Analysis root is not set, can not change directory")
    }

    setwd(file.path(project_path, analysis_root))
  }

  new(
    "Database",
    socket = socket,
    root_path = root_path,
    project = project_id,
    data_root = data_root,
    root = root,
    root_id = root_id,
    creator = creator
  )
}

#' Initialize a database in a production environment.
#'
#' @param socket ZMQ socket to use.
#' @param project Project id.
#' @param root Root container graph path.
#' @param chdir Change directory to project analyses folder.
database_prod <- function(socket, project, root, chdir) {
  cmd <- sprintf('{"Project": {"GetById": "%s"}}', project)
  project_data <- send_cmd(socket, cmd, result = FALSE)
  if (is.null(project_data)) {
    stop("Could not get project")
  }

  project_path <- project_data[[1]]
  project_data <- project_data[[2]]
  project_properties <- project_data$properties$Ok
  if (is.null(project_properties)) {
    stop("Project properties are not valid")
  }

  if (SYSNAME == "Windows") {
    ROOT_DIR <- "\\"
  } else {
    ROOT_DIR <- "/"
  }
  if (!startsWith(root, ROOT_DIR)) {
    stop(sprintf("Invalid path for %s", CONTAINER_ID_KEY))
  }
  container_graph_path <- substring(root, nchar(ROOT_DIR) + 1, nchar(root))
  if (SYSNAME == "Windows") {
    data_root <- join_path_windows(project_path, project_properties$data_root)
    root_path <- join_path_windows(data_root, container_graph_path)
  } else {
    data_root <- file.path(project_path, project_properties$data_root)
    root_path <- file.path(data_root, container_graph_path)
  }

  root_container <- get_container(socket, project, root)
  if (is.null(root_container)) {
    stop("Could not get root container")
  }

  root_properties <- root_container$properties$Ok
  if (is.null(root_properties)) {
    stop(
      "Root container properties file is corrupt:",
      root_container$properties$Err
    )
  }
  root_id <- root_properties$rid

  creator <- syre_analysis_id()
  if (is.na(creator)) {
    stop(sprintf("`%s` not set", ANALYSIS_ID_KEY))
  }
  creator <- list(Script = creator)

  if (chdir) {
    analysis_root <- project_properties$analysis_root
    if (is.null(analysis_root)) {
      stop("Analysis root is not set, can not change directory")
    }

    setwd(file.path(project_path, analysis_root))
  }

  new(
    "Database",
    socket = socket,
    root_path = root_path,
    project = project,
    data_root = data_root,
    root = root,
    root_id = root_id,
    creator = creator
  )
}

#' Gets the root Container of the database.
#'
#' @param db Syre database connection.
#'
#' @returns Root Container.
#' @export
#'
#' @examples
#' db <- database()
#' root <- root(db)
root <- function(db) {
  cmd <- sprintf(
    '{"Container": {"GetForAnalysis": {"project": "%s", "container": "%s"}}}',
    db@project,
    escape_str(db@root)
  )
  root <- send_cmd(db@socket, cmd)
  if (is.null(root$Ok)) {
    stop("could not get root container: ", root$Err)
  }

  container_from_search_json(root$Ok)
}

#' Find Containers matching the given filter criteria.
#'
#' @param db Syre database connection.
#' @param name Name of the Container to match.
#' @param type Type of the Container to match.
#' @param tags List of tags the Container has to match.
#' @param metadata Named list of metadata the Container has to match.
#'
#' @returns List of Containers matching the filter.
#' @export
#'
#' @examples
#' db <- database()
#' containers <- find_containers(db, type = "my_container")
find_containers <- function(
    db,
    name = NULL,
    type = NULL,
    tags = NULL,
    metadata = NULL) {
  if (is.null(tags)) {
    tags <- list()
  }
  if (is.null(metadata)) {
    metadata <- list()
  }

  query <- to_json(list(
    name = name,
    kind = type,
    tags = tags,
    metadata = metadata
  ))

  cmd <-
    sprintf(
      '{"Container": {"Search": { "project": "%s", "root": "%s", "query": %s}}}',
      db@project,
      escape_str(db@root),
      query
    )
  containers <- send_cmd(db@socket, cmd)
  containers |> map(container_from_search_json)
}

#' Finds a single Container matching the given filter criteria.
#' If multiple matching Containers are found, a random one is returned.
#'
#' @param db Syre database connection.
#' @param name Name of the Container to match.
#' @param type Type of the Container to match.
#' @param tags List of tags the Container has to match.
#' @param metadata Named list of metadata the Container has to match.
#'
#' @returns Single Container matched by the filter or `NULL` if none exist.
#' @export
#'
#' @examples
#' db <- database()
#' container <- find_container(db, name = "My Container")
find_container <- function(
    db,
    name = NULL,
    type = NULL,
    tags = NULL,
    metadata = NULL) {
  containers <-
    find_containers(
      db,
      name = name,
      type = type,
      tags = tags,
      metadata = metadata
    )
  if (length(containers) > 0) {
    return(containers[[1]])
  } else {
    NULL
  }
}

#' Find Assets matching the given filter criteria.
#'
#' @param db Syre database connection.
#' @param name Name of the Asset to match.
#' @param type Type of the Asset to match.
#' @param tags List of tags the Asset has to match.
#' @param metadata Named list of metadata the Asset has to match.
#'
#' @returns List of Assets matching the filter.
#' @export
#'
#' @examples
#' db <- database()
#' assets <- find_assets(db, type = "my_asset")
find_assets <- function(
    db,
    name = NULL,
    type = NULL,
    tags = NULL,
    metadata = NULL) {
  if (is.null(tags)) {
    tags <- list()
  }
  if (is.null(metadata)) {
    metadata <- list()
  }
  query <- to_json(list(
    name = name,
    kind = type,
    tags = tags,
    metadata = metadata
  ))

  cmd <- sprintf(
    '{"Asset": {"Search": {"project": "%s", "root": "%s", "query": %s}}}',
    db@project,
    escape_str(db@root),
    query
  )
  assets <- send_cmd(db@socket, cmd)
  assets |> map(asset_from_json)
}

#' Finds a single Asset matching the given filter criteria.
#' If multiple matching Assets are found, a random one is returned.
#'
#' @param db Syre database connection.
#' @param name Name of the Asset to match.
#' @param type Type of the Asset to match.
#' @param tags List of tags the Asset has to match.
#' @param metadata Named list of metadata the Asset has to match.
#'
#' @returns A single Asset, or `NULL` if none exist.
#' @export
#'
#' @examples
#' db <- database()
#' asset <- find_asset(db, name = "My Asset")
find_asset <- function(
    db,
    name = NULL,
    type = NULL,
    tags = NULL,
    metadata = NULL) {
  assets <-
    find_assets(
      db,
      name = name,
      type = type,
      tags = tags,
      metadata = metadata
    )
  if (length(assets) > 0) {
    return(assets[[1]])
  } else {
    NULL
  }
}

#' Adds an Asset to the Syre project.
#' The associated data should be saved at the return path.
#'
#' @param db Syre database connection.
#' @param file File name of the associated data.
#'  Must be a relative path.
#' @param name Name of the Asset.
#' @param type Type of the Asset.
#' @param description Description of the Asset.
#' @param tags List of tags for the Asset.
#' @param metadata Named list of metadata for the Asset.
#'
#' @returns Path to save the Asset's related data to.
#' @export
#'
#' @examples
#' db <- database()
#' path <- add_asset(db, "my_file.txt", name = "My Text File")
#' cat("Hello!", path)
add_asset <- function(
    db,
    file,
    name = NULL,
    type = NULL,
    description = NULL,
    tags = list(),
    metadata = list()) {
  if (isAbsolutePath(file)) {
    stop("file must be a relative path")
  }

  asset <- new_asset(
    file,
    creator = db@creator,
    name = name,
    type = type,
    tags = tags,
    metadata = metadata
  )

  assets_file <- assets_file_of(db@root_path)
  assets <- fromJSON(assets_file, simplifyVector = FALSE)
  dirty <- FALSE
  stored_asset <- NA
  for (idx in seq_along(assets)) {
    if (assets[[idx]]$path == asset$path) {
      stored_asset <- idx
      break
    }
  }

  if (is.na(stored_asset)) {
    assets[[length(assets) + 1]] <- asset
    dirty <- TRUE
  } else {
    if (!identical(asset$properties, assets[[stored_asset]]$properties)) {
      assets[[stored_asset]]$properties <- asset$properties
      dirty <- TRUE
    }
  }

  if (dirty) {
    json <- toJSON(unname(assets), auto_unbox = TRUE, null = "null", pretty = 2)
    json <- json_empty_list_to_obj("metadata", json)
    write(json, file = assets_file)
  }

  if (SYSNAME == "Windows") {
    join_path_windows(db@root_path, asset$path[[1]])
  } else {
    file.path(db@root_path, asset$path)
  }
}

#' Flags a resource (Container or Asset).
#'
#' @param db Syre database connection.
#' @param resource Resource to flag.
#' @param message Message to display.
#' @param severity Flag's everity.
#'
#' @export
#'
#' @examples
#' db <- database()
#' asset <- db |> find_asset()
#' db |> flag(asset, "Check me!")
flag <- function(db, resource, message, severity = c("warning", "info", "error")) {
  severity <- match.arg(severity)
  switch(severity,
    "warning" = severity <- "Warning",
    "info" = severity <- "Info",
    "error" = severity <- "Error",
    stop("unreachable")
  )

  resource_class <- class(resource)
  switch(resource_class,
    "Container" = container_rid <- resource@.rid,
    "Asset" = {
      parent <- db |> parent(resource)
      container_rid <- parent@.rid
    },
    stop("Invalid resource. Must be a Container or Asset.")
  )

  cmd <- sprintf(
    '{"Container": {"SystemPathById": {"project": "%s", "container": "%s"}}}',
    db@project,
    container_rid
  )
  container_path <- send_cmd(db@socket, cmd, result = FALSE)
  if (is.null(container_path)) {
    stop("Error setting flag: Could not get container path.")
  }

  switch(resource_class,
    "Container" = resource_container_path <- "/",
    "Asset" = resource_container_path <- relpath(resource@file, container_path),
    stop("unreachable")
  )

  flags_path <- flags_file_of(container_path)
  if (!file.exists(flags_path) || file.size(flags_path) == 0L) {
    flags <- list()
  } else {
    flags <- fromJSON(flags_path, simplifyVector = FALSE)
    # TODO: Ensure valid flags
  }

  flag <- list(
    id = uuid::UUIDgenerate(),
    message = message,
    severity = severity
  )
  resource_flags <- flags[[resource_container_path]]
  if (is.null(resource_flags[[1]])) {
    flags[[resource_container_path]] <- list(flag)
  } else {
    flags[[resource_container_path]][[length(resource_flags) + 1]] <- flag
  }

  write(toJSON(flags, auto_unbox = TRUE, pretty = 2), file = flags_path)
}
