//! Common functions.
use std::path::{Component, Path, PathBuf, StripPrefixError};
use syre_local as local;

#[cfg(any(feature = "client", feature = "server"))]
use crate::constants::{PortNumber, DATASTORE_PORT, LOCALHOST, PUB_SUB_PORT, REQ_REP_PORT};

/// Returns a localhost tcp address with the given port.
#[cfg(any(feature = "client", feature = "server"))]
pub fn localhost_with_port(port: PortNumber) -> String {
    format!("tcp://{LOCALHOST}:{port}")
}

/// Returns the URL of the ZMQ socket of the given type.
///
/// # Returns
/// `None` if the given socket type is not supported.
#[cfg(any(feature = "client", feature = "server"))]
pub fn zmq_url(kind: zmq::SocketType) -> Option<String> {
    let port = match kind {
        zmq::SocketType::REP => REQ_REP_PORT,
        zmq::SocketType::REQ => REQ_REP_PORT,
        zmq::SocketType::PUB => PUB_SUB_PORT,
        zmq::SocketType::SUB => PUB_SUB_PORT,
        _ => return None,
    };

    Some(localhost_with_port(port))
}

#[cfg(any(feature = "client", feature = "server"))]
pub fn datastore_url() -> String {
    format!("{LOCALHOST}:{DATASTORE_PORT}")
}

/// Prepend the root dir to the path.
/// If the path already begins with the root dir, the original path is returned.
pub fn prepend_root_dir(path: impl AsRef<Path>) -> PathBuf {
    use std::path::Component::RootDir;

    let path = path.as_ref();
    if let Some(RootDir) = path.components().next() {
        path.to_path_buf()
    } else {
        std::iter::once(RootDir).chain(path.components()).collect()
    }
}

/// # Returns
/// `true` if the path starts from root (`/`), `false` otherwise.
/// `false` if path is empty.
pub fn is_root_path(path: impl AsRef<Path>) -> bool {
    let Some(first) = path.as_ref().components().next() else {
        return false;
    };
    matches!(first, std::path::Component::RootDir)
}

/// Creates the absolute path from the data root to the container.
///
/// # Arguments
/// 1. `data_root`: Absolute path from the file system root to the data root.
/// 2. `container`: Absolute path from the file system root to the container.
///
/// # Panics
/// If either path is not absolute.
///
/// # Examples
/// ```rust
/// let data_root = "/user/syre/project/data"
/// let container = "/user/syre/project/data/child/grandchild"
///
/// assert_eq!(container_graph_path(&data_root, &data_root), "/");
/// assert_eq!(container_graph_path(&data_root, &container), "/child/grandchild");
/// ```
///
/// # See also
/// + [`container_system_path`]
pub fn container_graph_path(
    data_root: impl AsRef<Path>,
    container: impl AsRef<Path>,
) -> Result<PathBuf, StripPrefixError> {
    assert!(data_root.as_ref().is_absolute());
    assert!(container.as_ref().is_absolute());

    let path = container.as_ref().strip_prefix(data_root.as_ref())?;
    Ok(Path::new(Component::RootDir.as_os_str()).join(path))
}

/// Creates the absolute path from the file system root to the container.
///
/// # Arguments
/// 1. `data_root`: Absolute path from the file system root to the data root.
/// 2. `container`: Absolute path from the file system root to the container.
///
/// # Panics
/// If either path is not absolute.
///
/// # Examples
/// ```rust
/// let data_root = "/user/syre/project/data"
/// let container = "/child/grandchild"
///
/// assert_eq!(container_system_path(&data_root, &data_root), data_root);
/// assert_eq!(container_system_path(&data_root, &container), "/user/syre/project/data/child/grandchild");
/// ```
///
/// # See also
/// + [`container_graph_path`]
pub fn container_system_path(data_root: impl AsRef<Path>, container: impl AsRef<Path>) -> PathBuf {
    assert!(data_root.as_ref().is_absolute());
    assert!(is_root_path(&container));

    data_root
        .as_ref()
        .components()
        .chain(container.as_ref().components().skip(1))
        .collect()
}
