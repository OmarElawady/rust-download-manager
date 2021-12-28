mod daemon1;
mod err;
mod http;
mod rest;
mod state;
mod types;

#[macro_use]
extern crate rocket;
use std::net::SocketAddr;
use crate::daemon1::client::DaemonClient;
use crate::http::HTTPClient;
use clap::{App, Arg, SubCommand};
use daemon1::ManagerDaemon;

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
        .subcommand(
            SubCommand::with_name("cancel")
                .arg(
                    Arg::with_name("name")
                        .value_name("name")
                        .required(true)
                        .help("download name to cancel"),
                )
                .about("show info about the download"),
        )
        .get_matches();

    let addr: SocketAddr = matches.value_of("addr").unwrap().parse()?;
    
    match matches.subcommand() {
        ("add", Some(matches)) => {
            match HTTPClient::new(&format!("http://{}", addr))
                .await?
                .add(matches.value_of("url").unwrap().into())
                .await
            {
                Ok(_) => println!("ok"),
                Err(e) => println!("{}", e),
            }
        }
        ("list", _) => match HTTPClient::new(&format!("http://{}", addr)).await?.list().await {
            Ok(v) => println!("{}", v),
            Err(e) => println!("{}", e),
        },
        ("info", Some(matches)) => {
            match HTTPClient::new(&format!("http://{}", addr))
                .await?
                .info(matches.value_of("name").unwrap().into())
                .await
            {
                Ok(v) => println!("{}", v),
                Err(e) => println!("{}", e),
            }
        }
        ("cancel", Some(matches)) => {
            match HTTPClient::new(&format!("http://{}", addr))
                .await?
                .cancel(matches.value_of("name").unwrap().into())
                .await
            {
                Ok(_) => println!("ok"),
                Err(e) => println!("{}", e),
            }
        }
        _ => {
            let (job_sender, job_receiver) = async_channel::unbounded();
            let workers = matches.value_of("workers").unwrap().parse()?;
            let d = ManagerDaemon::new(workers, job_receiver)?;
            tokio::spawn(d.serve()); // TODO: revise waiting and such
            let figment = rocket::Config::figment()
            .merge(("address", addr.ip()))
            .merge(("port", addr.port()));
            rocket::custom(figment)
                .mount("/api/v1/jobs/", routes![rest::list, rest::info, rest::add, rest::cancel])
                .register("/", catchers![rest::internal_server_error, rest::not_found])
                .manage(DaemonClient { ch: job_sender })
                .launch()
                .await?;
        }
    }
    Ok(())
}
