use crate::Result;
use clap::{ArgGroup, Args, Subcommand};
use syre_core::types::UserId;
mod commands;

pub fn main(args: UserArgs) -> Result {
    match args.command {
        Command::List => commands::list(),
        Command::Add(user) => commands::add(user),
        Command::Delete(user) => commands::delete(user),
        Command::Edit(e_args) => {
            let name = match e_args.name {
                None => None,
                Some(name) if name.trim().is_empty() => Some(None),
                Some(name) => Some(Some(name)),
            };

            let edits = EditUserFields {
                name,
                email: e_args.email,
            };

            commands::edit(e_args.id, edits)
        }
    }
}

#[derive(Debug, Args)]
pub struct UserArgs {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    List,
    Add(AddArgs),
    Delete(UserId),
    Edit(EditArgs),
}

#[derive(Debug, Args)]
pub struct AddArgs {
    #[clap(short, long)]
    name: Option<String>,

    #[clap(short, long)]
    email: String,
}

#[derive(Debug, Args)]
#[clap(group(
        ArgGroup::new("edit")
        .required(true)
        .multiple(true)
))]
pub struct EditArgs {
    id: UserId,

    #[clap(short, long, group = "edit")]
    email: Option<String>,

    #[clap(short, long, group = "edit")]
    name: Option<String>,
}

#[derive(Debug)]
pub struct EditUserFields {
    email: Option<String>,
    name: Option<Option<String>>,
}
