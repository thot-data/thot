# constants
LOCALHOST <- "127.0.0.1"
SYRE_PORT <- 7047
SYRE_TIMEOUT <- 1000L # in milliseconds

#' Create a ZMQ socket.
#'
#' @returns ZMQ socket.
zmq_socket <- function() {
  socket <- init.socket(SYRE_ZMQ_CONTEXT, "ZMQ_REQ")
  set.send.timeout(socket, SYRE_TIMEOUT)
  set.linger(socket, SYRE_TIMEOUT)
  connect.socket(socket, sprintf("tcp://%s:%d", LOCALHOST, SYRE_PORT))

  socket
}

#' Convert an object into JSON.
#'
#' @param obj Object to convert.
#'
#' @returns JSON encoding of the object.
to_json <- function(obj) {
  toJSON(obj, auto_unbox = TRUE, null = "null")
}

#' Get the path to the correct database server.
#'
#' @returns Path to the local database executable for the current system.
database_server_path <- function() {
  sys_info <- Sys.info()
  exe <- switch(sys_info["sysname"],
    "Linux" = "x86_64-unknown-linux-gnu",
    "Darwin" = paste(sys_info["machine"], "-apple-darwin", sep = ""),
    "Windows" = "x86_64-pc-windows-msvc.exe",
  )

  exe <- paste("syre-local-database-", exe, sep = "")
  system.file(exe, package = "syre", mustWork = TRUE)
}

#' Escape special string characters.
#' Characters escaped:
#' + Backslash (\)
#'
#' @param val Sring to escape.
#'
#' @returns Escaped string.
escape_str <- function(val) {
  gsub("\\\\", "\\\\\\\\", val)
}

#' Split a path into its parts.
#'
#' @param path Path to split.
#'
#' @returns List of path parts.
split_path <- function(path) {
  if (dirname(path) == path) {
    path
  } else {
    c(split_path(dirname(path)), basename(path))
  }
}


#' @param path Base path.
#' @param start Prefix.
#'
#' @returns Relative path from `start` to `path`.

relpath <- function(path, start) {
  WINDOWS_UNC_PREFIX <- "^\\\\\\\\\\?\\\\"
  path <- gsub(WINDOWS_UNC_PREFIX, "", path)
  start <- gsub(WINDOWS_UNC_PREFIX, "", start)
  path_parts <- split_path(path)
  start_parts <- split_path(start)
  
  for (idx in seq_along(start_parts)) {
    if (start_parts[idx] != path_parts[idx]) {
      stop("start does not prefix path")
    }
  }

  idx_start <- length(start_parts) + 1
  rel_path_parts <- path_parts[idx_start:length(path_parts)]
  do.call(file.path, as.list(rel_path_parts))
}

#' Normalize Windows paths.
#' If total path length is less than 256, returns the normalized path,
#' otherwise returns the device (UNC) path.
#'
#' @param p1 Base path.
#' @param p2 Extension path.
#'
#' @returns Normalized path.
join_path_windows <- function(p1, p2) {
  len <- nchar(p1) + nchar(p2)
  if (len < 256) {
    return(normalizePath(file.path(p1, p2), mustWork = FALSE))
  }

  WINDOWS_UNC_PREFIX <- "\\\\?\\"
  if (!startsWith(p1, WINDOWS_UNC_PREFIX)) {
    p1 <- paste(WINDOWS_UNC_PREFIX, p1, sep = "")
  }

  if (!endsWith(p1, "\\")) {
    # no slashes
    p1 <- paste(p1, "\\\\", sep = "")
  } else if (!endsWith(p1, "\\\\")) {
    # one slash
    p1 <- paste(p1, "\\", sep = "")
  }

  if (startsWith(p2, "\\\\")) {
    p2 <- substring(p2, 2)
  } else if (startsWith(p2, "\\")) {
    p2 <- substring(p2, 1)
  }

  paste(p1, p2)
}

#' Datetime in ISO 8601 format.
utc_now <- function() {
  now <- as.POSIXlt(Sys.time(), tz = "UTC")
  strftime(now, "%Y-%m-%dT%H:%M:%SZ", tz = "UTC")
}

#' Converts an empty object (`[]`) in JSON to an empty object (`{}`)
#'
#' @param keys List of or singular object key(s) to convert.
#' @param json JSON in which to convert.
#' @returns JSON with key replaced as empty object if it was an empty list.
#'
#' @examples
#' my_json <- '{"my_obj": [], "my_num": 4}'
#' json_empty_list_to_obj("my_obj", my_json)
json_empty_list_to_obj <- function(keys, json) {
  if (!is.list(keys)) {
    keys <- list(keys)
  }

  for (key in keys) {
    search <- sprintf('"%s":\\s*\\[[\\r\\n\\s]*\\]', key)
    replace <- sprintf('"%s":{}', key)
    json <- gsub(search, replace, json, perl = TRUE)
  }

  json
}
