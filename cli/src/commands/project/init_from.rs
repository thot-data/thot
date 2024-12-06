use crate::Result;
use clap::Args;
use std::env;
use std::path::PathBuf;
use syre_local::project::project;

#[derive(Debug, Args)]
pub struct InitFromArgs {
    #[clap(short, long)]
    root: Option<PathBuf>,

    /// Relative path to the data root.
    #[clap(long, default_value = "data")]
    data_root: PathBuf,

    /// Relative path to the analysis root.
    #[clap(long, default_value = "analysis")]
    analysis_root: PathBuf,

    #[clap(long)]
    no_scripts: bool,
}

/// Initializes a new Syre project.
/// Initilizes and registers the project.
/// Initializes any existing folders as the `Container` graph moving them into the data root folder.
/// If analysis root exists, registers all valid files as scripts.
///
/// Searches for `data` folder to be used as the `Project`'s data root,
/// and `analysis` folder for the analysis root, creating them if they don't exist.
/// If the data root folder does not exist, move the contents of the folder into it.
/// e.g.
/// **Original**
/// my_project/
///     |- root_data.csv
///     |- group_1/
///     |   |- group_1_data.csv
///
/// **Initialized**
/// my_project/
///     |- analysis/
///     |- data/
///     |   |- root_data.csv
///     |   |- group_1
///     |   |   |- group_1_data.csv
///
pub fn main(args: InitFromArgs) -> Result {
    let root = match args.root {
        Some(root) => root.clone(),
        None => match env::current_dir() {
            Ok(dir) => dir,
            Err(err) => return Err(err.into()),
        },
    };

    let mut converter = project::converter::Converter::new();
    converter.set_data_root(&args.data_root)?;
    if args.no_scripts {
        converter.without_scripts();
    } else {
        converter.set_analysis_root(&args.analysis_root)?;
    }

    converter.convert(&root)?;
    tracing::info!("Initialized {root:?} as a Syre project.");

    Ok(())
}
