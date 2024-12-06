use clap::error::Error as ClapError;
use std::io;
use std::result::Result as StdResult;
use syre_core::Error as CoreError;
use syre_local::Error as LocalError;

// *************
// *** Error ***
// *************

#[derive(Debug)]
pub enum Error {
    Clap(ClapError),
    Core(CoreError),
    Io(io::Error),
    Local(LocalError),
    LoadContainer(syre_local::loader::container::State),
    LoadTree(syre_local::loader::error::tree::Error),
    IoSerde(syre_local::error::IoSerde),
}

impl From<ClapError> for Error {
    fn from(err: ClapError) -> Self {
        Self::Clap(err)
    }
}

impl From<CoreError> for Error {
    fn from(err: CoreError) -> Self {
        Self::Core(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<LocalError> for Error {
    fn from(err: LocalError) -> Self {
        Self::Local(err)
    }
}

impl From<syre_local::loader::error::container::Error> for Error {
    fn from(value: syre_local::loader::error::container::Error) -> Self {
        Self::LoadContainer(value)
    }
}

impl From<syre_local::loader::error::tree::Error> for Error {
    fn from(value: syre_local::loader::error::tree::Error) -> Self {
        Self::LoadTree(value)
    }
}

impl From<syre_local::error::IoSerde> for Error {
    fn from(value: syre_local::error::IoSerde) -> Self {
        Self::IoSerde(value)
    }
}

pub type Result<T = ()> = StdResult<T, Error>;
