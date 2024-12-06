use crate::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;
mod commands;

pub fn main(args: ContainerArgs) -> Result {
    match args.command {
        Command::Init(init_args) => commands::init(init_args),
        Command::New(new_args) => commands::new(new_args),
    }
}

#[derive(Debug, Args)]
pub struct ContainerArgs {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init(InitArgs),
    New(NewArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[clap(short, long)]
    path: Option<PathBuf>,

    /// Do not add files as Assets.
    #[clap(long)]
    no_assets: bool,

    /// Do not recurse.
    #[clap(long)]
    no_recurse: bool,
}

#[derive(Debug, Args)]
pub struct NewArgs {
    path: PathBuf,
}
