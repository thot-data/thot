use clap::{Parser, Subcommand};
use notify::Watcher;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use syre_local_database::constants;

const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Parser, Debug)]
#[clap(name = "Syre Local Database CLI", author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Pub => publ().unwrap(),
        Command::Sub => sub().unwrap(),
        Command::Req => req().unwrap(),
        Command::Rep => rep().unwrap(),

        Command::WatchFs { path, no_debounce } => {
            if no_debounce {
                watch_file_system(&path);
            } else {
                watch_file_system_debounce(&path);
            }
        }
    }
}

fn publ() -> zmq::Result<()> {
    let zmq_context = zmq::Context::new();
    let zmq_socket = zmq_context.socket(zmq::PUB).unwrap();
    zmq_socket
        .bind(&syre_local_database::common::zmq_url(zmq::PUB).unwrap())
        .unwrap();

    let stdin = io::stdin();
    let mut message = String::new();
    loop {
        stdin.read_line(&mut message).unwrap();
        zmq_socket.send(&constants::PUB_SUB_TOPIC, zmq::SNDMORE)?;
        zmq_socket.send(&message, 0)?;
        message.clear();
    }
}

fn sub() -> zmq::Result<()> {
    let zmq_context = zmq::Context::new();
    let zmq_socket = zmq_context.socket(zmq::SUB).unwrap();
    zmq_socket.connect(&syre_local_database::common::zmq_url(zmq::SUB).unwrap())?;
    zmq_socket
        .set_subscribe(constants::PUB_SUB_TOPIC.as_bytes())
        .unwrap();

    loop {
        let messages = match zmq_socket.recv_multipart(0) {
            Ok(msg) => msg,
            Err(err) => {
                tracing::debug!(?err);
                continue;
            }
        };

        let messages = messages
            .into_iter()
            .map(|msg| zmq::Message::try_from(msg).unwrap())
            .collect::<Vec<_>>();

        let topic = messages.get(0).unwrap().as_str().unwrap();
        let mut message = String::new();
        for msg in messages.iter().skip(1) {
            message.push_str(msg.as_str().unwrap());
        }

        match serde_json::from_str::<Vec<syre_local_database::event::Update>>(&message) {
            Ok(message) => println!(
                "{topic}\n{}\n",
                serde_json::to_string_pretty(&message).unwrap()
            ),

            Err(err) => println!("[could not decode: {err:?}]\n{topic}\n{message:?}\n"),
        }
    }
}

fn req() -> zmq::Result<()> {
    let zmq_context = zmq::Context::new();
    let zmq_socket = zmq_context.socket(zmq::REQ)?;
    zmq_socket.connect(&syre_local_database::common::zmq_url(zmq::REQ).unwrap())?;

    let stdin = io::stdin();
    let mut message = String::new();
    loop {
        stdin.read_line(&mut message).unwrap();
        zmq_socket.send(&message, 0)?;
        message.clear();
    }
}

fn rep() -> zmq::Result<()> {
    let zmq_context = zmq::Context::new();
    let zmq_socket = zmq_context.socket(zmq::REP).unwrap();
    zmq_socket.bind(&syre_local_database::common::zmq_url(zmq::REP).unwrap())?;

    loop {
        let msg = zmq_socket.recv_msg(0)?;
        println!("{:?}", msg.as_str().unwrap());
    }
}

fn watch_file_system(path: impl AsRef<Path>) {
    let mut watcher = notify::recommended_watcher(|res| match res {
        Ok(event) => println!("{event:?}"),
        Err(err) => println!("ERROR {err:?}"),
    })
    .unwrap();

    watcher
        .watch(path.as_ref(), notify::RecursiveMode::Recursive)
        .unwrap();

    loop {}
}

#[cfg(target_os = "macos")]
fn watch_file_system_debounce(path: impl AsRef<Path>) {
    let path = path.as_ref();
    let mut watcher: notify_debouncer_full::Debouncer<notify::PollWatcher, _> = {
        let config = notify::Config::default()
            .with_poll_interval(DEBOUNCE_TIMEOUT)
            .with_compare_contents(true);

        notify_debouncer_full::new_debouncer_opt(
            DEBOUNCE_TIMEOUT,
            None,
            move |events: notify_debouncer_full::DebounceEventResult| {
                println!("{events:?}");
            },
            notify_debouncer_full::FileIdMap::new(),
            config,
        )
        .unwrap()
    };

    watcher
        .watcher()
        .watch(path, notify::RecursiveMode::Recursive)
        .unwrap();

    watcher
        .cache()
        .add_root(path, notify::RecursiveMode::Recursive);

    loop {}
}

#[cfg(not(target_os = "macos"))]
fn watch_file_system_debounce(path: impl AsRef<Path>) {
    let path = path.as_ref();

    let mut watcher = notify_debouncer_full::new_debouncer(
        DEBOUNCE_TIMEOUT,
        None,
        move |events: notify_debouncer_full::DebounceEventResult| {
            println!("{events:?}");
        },
    )
    .unwrap();

    watcher
        .watcher()
        .watch(path, notify::RecursiveMode::Recursive)
        .unwrap();

    watcher
        .cache()
        .add_root(path, notify::RecursiveMode::Recursive);

    loop {}
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Publish an event to the database channel.
    Pub,

    /// Listen to published events from the database.
    Sub,

    /// Send a request to the database.
    Req,

    /// Listen for requests to the database.
    Rep,

    /// Listen to file system events.
    WatchFs {
        path: PathBuf,

        #[clap(long)]
        no_debounce: bool,
    },
}
