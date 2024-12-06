use crate::Result;
use clap::Args;
use std::env;
use std::path::{Path, PathBuf};
use syre_local::project::project;

#[derive(Debug, Args)]
pub struct NewArgs {
    name: String,

    #[clap(short, long)]
    root: Option<PathBuf>,
}

/// Creates a new Syre project.
pub fn main(args: NewArgs) -> Result {
    // get root path
    let root = match args.root {
        Some(p) => p,
        None => env::current_dir()?,
    };

    let root = root.join(Path::new(&args.name));

    // create project
    if let Err(err) = project::new(root.as_path()) {
        return Err(err.into());
    };

    // set project properties
    // project and root container: creator, name, permissions, lid
    tracing::info!("New project created at {:?}", &root);
    Ok(())
}
