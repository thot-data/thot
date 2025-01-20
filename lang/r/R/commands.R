#' Send a command to the database.
#'
#' @param socket ZMQ socket.
#' @param cmd Command to send.
#' @param result Handle response as a Result.
#'
#' @returns Deserialzed result.
send_cmd <- function(socket, cmd, result = TRUE) {
  sent <- send.raw.string(socket, cmd)
  if (!sent) {
    stop("Could not send message.")
  }

  res <- receive.string(socket)
  res <- fromJSON(res, simplifyVector = FALSE)
  if (result) {
    err <- res$Err
    if (!is.null(err)) {
      stop(err)
    }

    res <- res$Ok
  }

  res
}

#' Checks if a Syre database is available.
#'
#' @returns Whether a Syre database is available.
database_available <- function() {
  server_up <- tryCatch({
    socket <- socketConnection(port = SYRE_PORT)
    close(socket)
    TRUE
  },
  error = function(cond) {
    # port not open, no chance for server
    FALSE
  },
  warning = function(cond) {
    FALSE
  })

  if (!server_up) {
    return(FALSE)
  }

  # check if database is responsive
  cmd <- '{"Config": "Id"}'
  id <- send_cmd(zmq_socket(), cmd, result = FALSE)
  id == "syre local database"
}

#' Loads a Syre project.
#'
#' @param socket ZMQ socket.
#' @param path Path to the Project.
#'
#' @returns List of Project properties.
load_project <- function(socket, path) {
  cmd <- sprintf('{"Project": {"Load": "%s"}}', path)
  send_cmd(socket, cmd)
}

#' Loads a Syre Project's graph.
#'
#' @param socket ZMQ socket.
#' @param project Project's id.
#'
#' @returns List of graph components.
load_graph <- function(socket, project) {
  cmd <- sprintf('{"Graph": {"Load": "%s"}}', project)
  send_cmd(socket, cmd)
}

#' Get a container.
#'
#' @param socket ZMQ socket.
#' @param project Project id.
#' @param container Graph path to project. (i.e. Root container has path `/`.)
#'
#' @returns List of Container properties.
get_container <- function(socket, project, container) {
  cmd <- sprintf(
    '{"Container": {"Get": {"project": "%s", "container": "%s"}}}',
    project,
    escape_str(container)
  )
  send_cmd(socket, cmd)
}

#' Gets a Syre Container's path.
#'
#' @param socket ZMQ socket.
#' @param id Container id.
#'
#' @returns Container's path.
container_path <- function(socket, id) {
  cmd <- sprintf('{"Container": {"Path": "%s"}}', id)
  send_cmd(socket, cmd, result = FALSE)
}

#' Gets a Syre Container from its path.
#'
#' @param socket ZMQ socket.
#' @param path Path to the Container.
#'
#' @returns List of Container properties.
container_by_path <- function(socket, path) {
  cmd <- sprintf('{"Container": {"ByPath": "%s"}}', path)
  send_cmd(socket, cmd, result = FALSE)
}
