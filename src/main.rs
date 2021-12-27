mod api;
mod daemon;
mod db;
mod err;
mod http;
mod rest;
mod types;
mod client;
mod daemon1;
mod state;

#[macro_use]
extern crate rocket;
use crate::api::{AckCommand, AddCommand, ErrorCommand, InfoCommand, ListCommand, Message};
use crate::http::HTTPClient;
use crate::http::HTTPListener;
use crate::http::HTTPState;
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

#[rocket::main]
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
            HTTPClient::new("http://localhost:8000")
                .await?
                .add(matches.value_of("url").unwrap().into())
                .await?;
            print!("ok")
        }
        ("list", _) => {
            print!(
                "{}",
                HTTPClient::new("http://localhost:8000")
                    .await?
                    .list()
                    .await?
            )
        }
        ("info", Some(matches)) => {
            print!(
                "{}",
                HTTPClient::new("http://localhost:8000")
                    .await?
                    .info(matches.value_of("name").unwrap().into())
                    .await?
            )
        }
        _ => {
            let (job_sender, job_receiver) = async_channel::unbounded();
            let http_listener = HTTPListener { ch: job_receiver };
            let workers = matches.value_of("workers").unwrap().parse().unwrap();
            let d = ManagerDaemon::new(workers, http_listener)?;
            tokio::spawn(d.serve()); // TODO: revise waiting and such
            rocket::build()
                .mount("/api/v1/jobs/", routes![rest::list, rest::info, rest::add])
                .manage(HTTPState { ch: job_sender })
                .launch()
                .await?;
        }
    }
    Ok(())
}
