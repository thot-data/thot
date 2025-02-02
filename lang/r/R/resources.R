setClassUnion("OptChar", c("character", "NULL"))

setClass("Database",
         slots = list(
           root = "character",
           root_path = "character",
           socket = "externalptr"
         ))

setClass(
  "Container",
  slots = list(
    .rid = "character",
    name = "OptChar",
    type = "OptChar",
    description = "OptChar",
    tags = "list",
    metadata = "list",
    assets = "list"
  )
)

setClass(
  "Asset",
  slots = list(
    .rid = "character",
    name = "OptChar",
    type = "OptChar",
    description = "OptChar",
    tags = "list",
    metadata = "list",
    file = "OptChar"
  )
)

#' Create a new Asset.
#'
#' @param rid Resource id.
#' @param file File path.
#' @param name Name.
#' @param type Type.
#' @param description Description.
#' @param tags List of tags.
#' @param metadata List of metadata.
Asset <- function(rid,
                  file,
                  name = NULL,
                  type = NULL,
                  description = NULL,
                  tags = list(),
                  metadata = list()) {
  new(
    "Asset",
    .rid = rid,
    file = file,
    name = name,
    type = type,
    description = description,
    tags = tags,
    metadata = metadata
  )
}

#' Create a new Container.
#'
#' @param rid Resource id.
#' @param name Name.
#' @param type Type.
#' @param description Description.
#' @param tags List of tags.
#' @param metadata List of metadata.
#' @param assets List of Assets.
Container <- function(rid,
                      name = NULL,
                      type = NULL,
                      description = NULL,
                      tags = list(),
                      metadata = list(),
                      assets = list()) {
  new(
    "Container",
    .rid = rid,
    name = name,
    type = type,
    description = description,
    tags = tags,
    metadata = metadata,
    assets = assets
  )
}

#' Converts a list of properties to an Asset.
#' The list should mirror the structure of the JSON representation of an Asset.
#'
#' @param asset List of properties.
#'
#' @returns Asset.
asset_from_json <- function(asset) {
  properties <- asset$properties
  Asset(
    rid = asset$rid,
    file = asset$path[[1]],
    name = properties$name,
    type = properties$kind,
    description = properties$description,
    tags = properties$tags,
    metadata = properties$metadata
  )
}

#' Converts a list of properties to a Container.
#' The list should mirror the structure of the JSON representation of a Container.
#'
#' @param container List of properties.
#'
#' @returns Container.
container_from_json <- function(container) {
  # convert assets
  assets <- container$assets |> map(asset_from_json)

  # convert container
  properties <- container$properties
  Container(
    rid = container$rid,
    name = properties$name,
    type = properties$kind,
    description = properties$description,
    tags = properties$tag,
    metadata = properties$metadata,
    assets = assets
  )
}

#' Gets the children of the Container.
#'
#' @param db Thot database connection.
#' @param container Thot Container.
#'
#' @returns List of the Container's children.
#' @export
#'
#' @examples
#' db <- database()
#' container <- db |> find_container(type = "child")
#' childs <- db |> children(container)
children <- function(db, container) {
  cmd <-
    sprintf('{"GraphCommand": {"Children": %s}}',
            to_json(container@.rid))

  children_ids <- send_cmd(db@socket, cmd, result = FALSE)
  childs <- vector("list", length(children_ids))
  for (i in seq_along(children_ids)) {
    cmd <-
      sprintf('{"ContainerCommand": {"GetWithMetadata": %s}}',
              to_json(children_ids[[i]]))

    container <- send_cmd(db@socket, cmd, result = FALSE)
    childs[[i]] <- container_from_json(container)
  }

  childs
}

#' Get the parent of a resource.
#'
#' @param db Thot database connection.
#' @param resource Thot resource.
#'
#' @returns Parent of the resource, or `NULL` if it does not exist in the current context.
#' @export
setGeneric("parent", function(db, resource)
  standardGeneric("parent"))

#' Gets the parent of the Container within the database context.
#'
#' @param db Thot database connection.
#' @param resource Thot Container.
#'
#' @returns Container's parent or `NULL` if the root of the database.
#' @export
#'
#' @examples
#' db <- database()
#' container <- db |> find_container(type = "child")
#' parent <- db |> parent(container)
setMethod("parent", signature(db = "Database", resource = "Container"), function(db, resource) {
  if (resource@.rid == db@root) {
    return(NULL)
  }

  cmd <-
    sprintf('{"GraphCommand": {"Parent": %s}}', to_json(resource@.rid))

  container <- send_cmd(db@socket, cmd, result = FALSE)

  cmd <-
    sprintf('{"ContainerCommand": {"GetWithMetadata": %s}}',
            to_json(container))

  container <- send_cmd(db@socket, cmd, result = FALSE)
  container_from_json(container)
})

#' Gets the parent of the Asset.
#'
#' @param db Thot database connection.
#' @param resource Thot Asset.
#'
#' @returns Asset's parent Container.
#' @export
#'
#' @examples
#' db <- database()
#' asset <- db |> find_asset(type = "data")
#' container <- db |> parent(asset)
setMethod("parent", signature(db = "Database", resource = "Asset"), function(db, resource) {
  cmd <-
    sprintf('{"AssetCommand": {"Parent": %s}}', to_json(resource@.rid))

  container <- send_cmd(db@socket, cmd, result = FALSE)

  cmd <-
    sprintf('{"ContainerCommand": {"GetWithMetadata": %s}}',
            to_json(container$rid))

  container <- send_cmd(db@socket, cmd, result = FALSE)
  container_from_json(container)
})
