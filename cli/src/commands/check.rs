//! Check a project's health.
use crate::Result;
use clap::Args;
use std::path::{Path, PathBuf};
use std::{env, fs};
use syre_local::loader::container::Loader as ContainerLoader;
use syre_local::project::container;

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[clap(long)]
    path: Option<PathBuf>,

    #[clap(long, default_value_t = true)]
    recurse: bool,
}

/// Check a path to ensure no files are corrupted.
pub fn main(args: CheckArgs) -> Result {
    let path = match args.path {
        Some(path) => path.clone(),
        None => match env::current_dir() {
            Ok(dir) => dir,
            Err(err) => return Err(err.into()),
        },
    };

    if check_path(path, args.recurse)? {
        println!("All ok");
    }

    Ok(())
}

/// Check the health of the path.
///
/// # Returns
/// + `true` if healthy, `false` otherwise.
fn check_path(path: impl AsRef<Path>, recurse: bool) -> Result<bool> {
    let path = path.as_ref();
    let mut is_healthy = true;
    if container::path_is_container(path) {
        match ContainerLoader::load(path) {
            Ok(_) => {}
            Err(err) => {
                is_healthy = false;
                println!("Could not load {path:?}: {err:?}");
            }
        }
    }

    if recurse {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.path().is_dir() {
                if check_path(entry.path(), recurse)? {
                    is_healthy = false;
                }
            }
        }
    }

    Ok(is_healthy)
}
