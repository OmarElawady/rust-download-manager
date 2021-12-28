mod err;
mod http;
mod jobs;
mod manager;
mod types;

#[macro_use]
extern crate rocket;
use crate::http::HTTPClient;
use crate::manager::client::ManagerClient;
use clap::{App, Arg, SubCommand};
use manager::ManagerDaemon;
use std::net::SocketAddr;

#[rocket::main]
async fn main() -> Result<(), err::ManagerError> {
    let matches = App::new("manager")
        .author("Omar Elawady (omarelawady1998@gmail.com)")
        .about("Download manager")
        .arg(
            Arg::with_name("addr")
                .value_name("addr")
                .short("a")
                .long("addr")
                .default_value("127.0.0.1:8000")
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
            Arg::with_name("downloads")
                .value_name("downloads")
                .short("d")
                .long("downloads")
                .default_value("~/Downloads")
                .help("directory to put downloads in"),
        )
        .arg(
            Arg::with_name("database")
                .value_name("database")
                .short("b")
                .long("database")
                .default_value("/tmp/downlaods.db")
                .help("path to persist download info"),
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
                .arg(
                    Arg::with_name("name")
                        .value_name("name")
                        .help("use this name instead of the last segment path"),
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
        .subcommand(
            SubCommand::with_name("cancel")
                .arg(
                    Arg::with_name("name")
                        .value_name("name")
                        .required(true)
                        .help("download name to cancel"),
                )
                .arg(
                    Arg::with_name("forget")
                        .short("f")
                        .long("forget")
                        .help("remove from the tracked downloads"),
                )
                .arg(
                    Arg::with_name("delete")
                        .short("d")
                        .long("delete")
                        .help("delete the underlying file on disk, implies forget"),
                )
                .about("show info about the download"),
        )
        .get_matches();

    let addr: SocketAddr = matches.value_of("addr").unwrap().parse()?;

    match matches.subcommand() {
        ("add", Some(matches)) => {
            match HTTPClient::new(&format!("http://{}", addr))
                .await?
                .add(matches.value_of("url").unwrap(), matches.value_of("name"))
                .await
            {
                Ok(_) => println!("ok"),
                Err(e) => println!("{}", e),
            }
        }
        ("list", _) => match HTTPClient::new(&format!("http://{}", addr))
            .await?
            .list()
            .await
        {
            Ok(v) => println!("{}", v),
            Err(e) => println!("{}", e),
        },
        ("info", Some(matches)) => {
            match HTTPClient::new(&format!("http://{}", addr))
                .await?
                .info(matches.value_of("name").unwrap())
                .await
            {
                Ok(v) => println!("{}", v),
                Err(e) => println!("{}", e),
            }
        }
        ("cancel", Some(matches)) => {
            match HTTPClient::new(&format!("http://{}", addr))
                .await?
                .cancel(
                    matches.value_of("name").unwrap(),
                    matches.is_present("forget"),
                    matches.is_present("delete"),
                )
                .await
            {
                Ok(_) => println!("ok"),
                Err(e) => println!("{}", e),
            }
        }
        _ => {
            let (job_sender, job_receiver) = async_channel::unbounded();
            let workers = matches.value_of("workers").unwrap().parse()?;
            let db_path = matches.value_of("database").unwrap();
            let downloads_path =
                shellexpand::tilde(matches.value_of("downloads").unwrap()).to_string();
            let d = ManagerDaemon::new(workers, job_receiver, db_path, &downloads_path)?;
            tokio::spawn(d.serve()); // TODO: revise waiting and such
            let figment = rocket::Config::figment()
                .merge(("address", addr.ip()))
                .merge(("port", addr.port()));
            rocket::custom(figment)
                .mount(
                    "/api/v1/jobs/",
                    routes![
                        http::rest::list,
                        http::rest::info,
                        http::rest::add,
                        http::rest::cancel
                    ],
                )
                .register(
                    "/",
                    catchers![http::rest::internal_server_error, http::rest::not_found],
                )
                .manage(ManagerClient { ch: job_sender })
                .launch()
                .await?;
        }
    }
    Ok(())
}
