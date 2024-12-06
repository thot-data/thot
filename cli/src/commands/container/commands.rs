use super::{InitArgs, NewArgs};
use crate::common::abs_path;
use crate::Result;
use std::env;
use syre_local::project::container;

pub fn init(args: InitArgs) -> Result {
    let path = match args.path {
        None => env::current_dir()?,
        Some(path) => path,
    };

    let mut builder = container::InitOptions::init();
    builder.recurse(!args.no_recurse);
    if args.no_assets {
        builder.without_assets();
    } else {
        builder.with_assets();
    }

    let rid = builder.build(&path)?;
    tracing::info!("Initialized {path:?} as a Container with {rid:?}");
    Ok(())
}

pub fn new(args: NewArgs) -> Result {
    let path = abs_path(args.path)?;
    let builder = container::InitOptions::new();
    builder.build(&path)?;
    tracing::info!("Initialized `{path:?}` as a Container");
    Ok(())
}
