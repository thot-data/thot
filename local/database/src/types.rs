//! Types.
use serde::{Deserialize, Serialize};

/// Convenience type to indicate valid port numbers.
pub type PortNumber = u16;

/// Socket types used for hash keys.
///
/// # Note
/// `zmq::SocketType` is not `hash`able, making this enum required.
/// This has been raised as an issue on the [`zmq GitHub`](https://github.com/erickt/rust-zmq/issues/362).
/// If this feature is implemented, this enum can be depricated.
#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Clone, Debug)]
pub enum SocketType {
    /// [`zmq::SocketType::REP`]
    REP,
}
