use crate::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use syre_core::types::UserId;
mod commands;

/// Configures a Syre project.
pub fn main(args: ConfigArgs) -> Result {
    // run command
    match args.command {
        Command::SetUser(user) => commands::set_active_user(&user),
    }
}

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[clap(subcommand)]
    command: Command,
    root: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    SetUser(UserId),
}
