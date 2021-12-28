use super::stream::ManagerStream;
use super::types::{AckCommand, CancelInfo, DownloadJob, InfoResponse, ListResponse, Message};
use super::worker::DownloadWorker;
use crate::err::ManagerError;
use crate::err::ManagerErrorKind::{
    DownloadJobNameAlreadyExist, DownloadJobNotFound, InvalidMessage,
};
use crate::jobs::client::StateClient;
use crate::jobs::state::StateDaemon;
use crate::types::{JobInfo, State};
use async_channel;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;
use tokio;
use tokio::sync::watch;
use tokio::time;
use url::Url;

use rand::{distributions::Alphanumeric, Rng}; // 0.8

pub struct ManagerDaemon {
    server: async_channel::Receiver<ManagerStream>,
    job_sender: async_channel::Sender<DownloadJob>,
    state_client: StateClient,
    cancel_channels: HashMap<String, watch::Sender<CancelInfo>>,
    downloads_path: String,
}

impl ManagerDaemon {
    // TODO: refactor into a config struct
    pub fn new(
        workers: u32,
        listener: async_channel::Receiver<ManagerStream>,
        db_path: &str,
        downloads_path: &str,
    ) -> Result<Self, ManagerError> {
        let (job_sender, job_receiver) = async_channel::unbounded();
        let (state_sender, state_receiver) = async_channel::unbounded();
        for _ in 0..workers {
            tokio::spawn(DownloadWorker::new(job_receiver.clone(), state_sender.clone()).work());
        }
        tokio::spawn(StateDaemon::new(state_receiver, db_path)?.work());
        Ok(ManagerDaemon {
            server: listener,
            job_sender,
            state_client: StateClient::new(state_sender),
            cancel_channels: HashMap::new(),
            downloads_path: downloads_path.into(),
        })
    }
    pub async fn serve(mut self) -> Result<(), ManagerError> {
        if let Err(e) = self.push_active_jobs().await {
            println!("failed to push active jobs {}", e)
        }
        // a single loop handling all the connections
        loop {
            let stream = self.server.recv().await;
            if let Err(e) = stream {
                println!("couldn't accept connection {}", e);
                time::sleep(time::Duration::from_secs(1)).await;
                continue;
            }
            let mut stream = stream.unwrap(); // safe unwrap
            let cmd = match self.handle(&mut stream).await {
                Err(e) => Message::Error(e),
                Ok(msg) => msg,
            };
            if let Err(e) = stream.write(&cmd).await {
                println!("error sending response {:?}: {}", cmd, e);
            }
            self.scrape_cancel_channels().await;
        }
    }
    async fn push_active_jobs(&mut self) -> Result<(), ManagerError> {
        let states = self.state_client.list().await?;
        for state in states.iter() {
            if state.state == State::Active {
                let (tx, rx) = watch::channel(CancelInfo {
                    cancel: false,
                    delete: false,
                });
                let job = DownloadJob {
                    name: state.name.clone(),
                    file_path: Path::new(&state.path).to_path_buf(),
                    url: state.url.to_string(),
                    cancel_channel: rx,
                };
                self.cancel_channels.insert(state.name.clone(), tx);
                if let Err(e) = self.job_sender.send(job).await {
                    println!("failed to send to the download channel {}", e);
                }
            }
        }
        Ok(())
    }
    async fn scrape_cancel_channels(&mut self) {
        // TODO: when things get too big, scrape on periods
        let mut to_remove = Vec::new();
        for (name, _) in self.cancel_channels.iter() {
            let state = self.state_client.get(name).await;
            match state {
                Err(e) => {
                    if e.kind == DownloadJobNotFound {
                        to_remove.push(name.clone());
                    } else {
                        println!("failed to get job from db to check its state {}", e)
                    }
                }
                Ok(state) => {
                    if state.state != State::Pending && state.state != State::Active {
                        to_remove.push(name.clone());
                    }
                }
            }
        }
        for name in to_remove.into_iter() {
            self.cancel_channels.remove(&name);
        }
    }
    async fn handle(&mut self, api: &mut ManagerStream) -> Result<Message, ManagerError> {
        let cmd = api.read().await?;
        match cmd {
            Message::Add(c) => {
                println!("adding {}", c.url);
                Ok(self.add(&c.url, c.name.as_deref()).await?)
            }
            Message::Info(c) => {
                println!("querying {}", c.name);
                Ok(self.info(&c.name).await?)
            }
            Message::Cancel(c) => {
                println!("cancelling");
                Ok(self.cancel(&c.name, c.forget, c.delete).await?)
            }
            Message::List(_) => {
                println!("listing");
                Ok(self.list().await?)
            }
            _ => {
                println!("thanks for the acknowledgment, don't expect it though");
                Ok(Message::Error(ManagerError {
                    kind: InvalidMessage,
                    msg: format!("unexpected command {:?}", cmd),
                }))
            }
        }
    }
    async fn job_exists(&self, name: &str) -> Result<bool, ManagerError> {
        let job = self.state_client.get(name).await;
        if let Err(e) = job {
            if e.kind == DownloadJobNotFound {
                Ok(false)
            } else {
                Err(e)
            }
        } else {
            Ok(true)
        }
    }
    async fn random_name() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect()
    }
    async fn add(&mut self, url: &str, mut name: Option<&str>) -> Result<Message, ManagerError> {
        println!("name: {:?}", name);
        let u = Url::parse(url)?;
        let segments = u.path_segments();
        if let Some(segments) = segments {
            let last = segments.last();
            if name.is_none() && !last.unwrap_or("").is_empty() {
                name = last;
            }
        }
        let rand_name = Self::random_name().await;
        let name = name.unwrap_or_else(|| rand_name.as_ref());
        if self.job_exists(name).await? {
            return Err(ManagerError {
                kind: DownloadJobNameAlreadyExist,
                msg: format!("{} already exists", name),
            });
        }
        let (tx, rx) = watch::channel(CancelInfo {
            cancel: false,
            delete: false,
        });
        let file_path = Path::new(&self.downloads_path).join(name);
        let job = DownloadJob {
            name: name.into(),
            // TODO: make configurable
            file_path: file_path.clone(),
            url: url.to_string(),
            cancel_channel: rx,
        };
        self.state_client
            .update(JobInfo {
                name: name.into(),
                url: url.into(),
                path: file_path
                    .to_str()
                    .unwrap_or("invalid path, shouldn't happen")
                    .into(),
                downloaded: 0,
                total: 0,
                state: State::Pending,
                msg: "".into(),
            })
            .await?;
        self.job_sender.send(job).await?;
        self.cancel_channels.insert(name.into(), tx);
        Ok(Message::Ack(AckCommand {}))
    }
    async fn list(&self) -> Result<Message, ManagerError> {
        Ok(Message::ListResponse(ListResponse::from(
            self.state_client.list().await?,
        )))
    }
    async fn info(&self, name: &str) -> Result<Message, ManagerError> {
        Ok(Message::InfoResponse(InfoResponse::from(
            &self.state_client.get(name).await?,
        )))
    }
    async fn cancel(
        &mut self,
        name: &str,
        forget: bool,
        delete: bool,
    ) -> Result<Message, ManagerError> {
        if let Some(ch) = self.cancel_channels.remove(name) {
            // deleting a non-existent job will succceed (bad?)
            ch.send(CancelInfo {
                cancel: true,
                delete,
            })?;
            self.state_client
                .update_job_state(name, State::Cancelled)
                .await?;
        }
        if delete {
            let state = self.state_client.get(name).await?;
            if let Err(e) = std::fs::remove_file(state.path) {
                if e.kind() != ErrorKind::NotFound {
                    return Err(e.into());
                }
            }
        }
        if delete || forget {
            if let Err(e) = self.state_client.delete(name).await {
                if e.kind != DownloadJobNotFound {
                    // abort on database errors only
                    // so that a previous forget that
                    // failed to do the rest can work
                    return Err(e);
                }
            }
        }
        Ok(Message::Ack(AckCommand))
    }
}
