PROJECT_ID_KEY <- "SYRE_PROJECT_ID"
CONTAINER_ID_KEY <- "SYRE_CONTAINER_ID"
ANALYSIS_ID_KEY <- "SYRE_ANALYSIS_ID"
APP_DIR <- ".syre"
ASSETS_FILE <- "assets.json"
FLAGS_FILE = "flags.json"

#' @param db Syre database connection.
#'
#' @returns Active user.
active_user <- function(socket) {
  config <- send_cmd(socket, '{"State": "LocalConfig"}')
  if (is.null(config)) {
    stop("Could not get local config")
  }

  config$user
}

#' Gets the `SYRE_PROJECT_ID` environment variable.
#'
#' @returns Project id or `NA`.
syre_project_id <- function() {
  Sys.getenv(PROJECT_ID_KEY, unset = NA)
}

#' Gets the `SYRE_CONTAINER_ID` environment variable.
#'
#' @returns Root container path or `NA`.
syre_container_path <- function() {
  Sys.getenv(CONTAINER_ID_KEY, unset = NA)
}

#' Gets the `SYRE_ANALYSIS_ID` environment variable.
#'
#' @returns Analysis id or `NA`.
syre_analysis_id <- function() {
  Sys.getenv(ANALYSIS_ID_KEY, unset = NA)
}

#' Gets the Project path given a path.
#' Returns `NULL` if the path is not in a project.
#'
#' @param path Path to get the Project root of.
#'
#' @returns Project path of the resource, or `NULL`.
project_resource_root_path <- function(path) {
  cmd <-
    sprintf(
      '{"Project": {"ResourceRootPath": "%s"}}',
      escape_str(path)
    )
  path <- send_cmd(zmq_socket(), cmd)
  path
}

#' @param base_path Base path of the container from the system root.
#'
#' @returns Path to the container's assets file.
assets_file_of <- function(base_path) {
  normalizePath(file.path(base_path, APP_DIR, ASSETS_FILE))
}

#' @param base_path Base path of the container from the system root.
#'
#' @returns Path to the container's flags file.
flags_file_of <- function(base_path) {
  normalizePath(file.path(base_path, APP_DIR, FLAGS_FILE))
}
#' Creates a new core Asset.
#' @param file File name of the associated data.
#'  Use relative paths to place the Asset in a bucket.
#' @param creator User.
#' @param name Name of the Asset to match.
#' @param type Type of the Asset to match.
#' @param tags List of tags the Asset has to match.
#' @param metadata Named list of metadata the Asset has to match.
#'
#' @returns New asset as a list.
new_asset <- function(
    file,
    creator,
    name = NULL,
    type = NULL,
    tags = list(),
    metadata = list()) {
  properties <- list(
    created = utc_now(),
    creator = creator,
    name = name,
    kind = type,
    description = NULL,
    tags = tags,
    metadata = metadata
  )

  list(
    rid = uuid::UUIDgenerate(),
    properties = properties,
    path = file
  )
}


#' Create a new `StandardProperties` list.
#'
#' @param creator User.
#' @param name Name of the Asset to match.
#' @param type Type of the Asset to match.
#' @param tags List of tags the Asset has to match.
#' @param metadata Named list of metadata the Asset has to match.
#'
#' @returns Standard properties as a named list.
AssetProperties <-
  function(creator,
           name = NULL,
           type = NULL,
           tags = list(),
           metadata = list()) {

  }
