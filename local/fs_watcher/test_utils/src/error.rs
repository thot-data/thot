/// Results and error.
use std::io;
use std::result::Result as StdResult;

// *************
// *** Error ***
// *************

/// Standard error type.
#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

// **************
// *** Result ***
// **************

pub type Result<T = ()> = StdResult<T, Error>;
