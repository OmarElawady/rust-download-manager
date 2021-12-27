mod api;
mod daemon;
mod err;
mod types;
mod db;

use crate::api::{Message, AddCommand, ListCommand, InfoCommand, AckCommand, ErrorCommand};
use clap::{App, Arg, SubCommand};
use daemon::ManagerDaemon;

async fn client_command(addr: &str, cmd: &Message) -> Result<(), err::ManagerError> {
    let mut cl = api::ManagerApi::new_client(addr).await?;
    cl.write(cmd).await?;
    let resp = cl.read().await?;
    match resp {
        Message::Ack(_) => println!("ok"),
        Message::Error(e) => println!("error: {}", e.msg),
        Message::ListResponse(e) => println!("{}", e),
        Message::InfoResponse(e) => println!("{}", e),
        _ => println!("server shouldn't send this: {:?}", resp),
    }
    cl.shutdown().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), err::ManagerError> {
    let matches = App::new("manager")
        .author("Omar Elawady (omarelawad11998@gmail.com)")
        .about("Download manager")
        .arg(
            Arg::with_name("addr")
                .value_name("addr")
                .short("a")
                .long("addr")
                .default_value("127.0.0.1:1984")
                .help("the address to bind to in case of a daemon, or to connect to otherwise"),
        )
        .arg(
            Arg::with_name("workers")
                .value_name("workers")
                .short("w")
                .long("workers")
                .default_value("5")
                .help("number of workers == max number of parallel downloads"),
        )
        .arg(
            Arg::with_name("debug")
                .short("d")
                .long("debug")
                .help("run in debug mode"),
        )
        .subcommand(SubCommand::with_name("list").about("list all downloads and their status"))
        .subcommand(
            SubCommand::with_name("add")
                .arg(
                    Arg::with_name("url")
                        .value_name("url")
                        .required(true)
                        .help("url to download"),
                )
                .about("add a new download job"),
        )
        .subcommand(
            SubCommand::with_name("info")
                .arg(
                    Arg::with_name("name")
                        .value_name("name")
                        .required(true)
                        .help("download name (e.g. file1.zip in http://example.com/file1.zip)"),
                )
                .about("show info about the download"),
        )
        .get_matches();

    let addr = matches.value_of("addr").unwrap();
    match matches.subcommand() {
        ("add", Some(matches)) => {
            client_command(
                addr,
                &Message::Add(AddCommand{url: matches.value_of("url").unwrap().into()}),
            )
            .await?;
        }
        ("list", _) => {
            client_command(
                addr,
                &Message::List(ListCommand),
            )
            .await?;
        }
        ("info", Some(matches)) => {
            client_command(
                addr,
                &Message::Info(InfoCommand{name: matches.value_of("name").unwrap().into()}),
            )
            .await?;
        }
        _ => {
            let workers = matches.value_of("workers").unwrap().parse().unwrap();
            let d = ManagerDaemon::new(addr, workers)?;
            d.serve().await?
        }
    }
    Ok(())
}
