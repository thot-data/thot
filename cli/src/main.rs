use clap::{Parser, Subcommand};
use syre_cli::commands::{check, config, container, project, run, user};

fn main() {
    let cli = Cli::parse();

    let res = match cli.command {
        // top level commands
        Command::New(args) => project::new::main(args),
        Command::Init(args) => project::init_from::main(args),
        Command::Move(args) => project::r#move::main(args),
        Command::Run(args) => run::main(args),
        Command::Check(args) => check::main(args),

        // subcommands
        Command::Config(args) => config::main(args),
        Command::User(args) => user::main(args),
        Command::Container(args) => container::main(args),
    };

    match res {
        Ok(()) => {}
        Err(err) => panic!("Something went wrong: {:?}", err),
    };
}

#[derive(Debug, Parser)]
#[clap(name = "Syre CLI")]
#[clap(version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    #[clap(short, long, global = true)]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    // top level commands
    New(project::new::NewArgs),
    Init(project::init_from::InitFromArgs),
    Run(run::RunArgs),
    Move(project::r#move::MoveArgs),
    Check(check::CheckArgs),

    // subcommands
    Config(config::ConfigArgs),
    User(user::UserArgs),
    Container(container::ContainerArgs),
}
