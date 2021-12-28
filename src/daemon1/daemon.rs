use std::collections::HashMap;
use super::stream::ManagerStream;
use super::types::{AckCommand, DownloadJob, InfoResponse, ListResponse, Message};
use super::worker::DownloadWorker;
use crate::err::ManagerError;
use crate::err::ManagerErrorKind::InvalidMessage;
use crate::err::ManagerErrorKind::DownloadJobNotFound;
use crate::state::client::StateClient;
use crate::state::state::StateDaemon;
use crate::types::{JobState, State};
use async_channel;
use std::path::Path;
use tokio;
use tokio::time;
use url::Url;
use tokio::sync::watch;

pub struct ManagerDaemon {
    server: async_channel::Receiver<ManagerStream>,
    job_sender: async_channel::Sender<DownloadJob>,
    state_client: StateClient,
    cancel_channels: HashMap<String, watch::Sender<bool>>
}

impl ManagerDaemon {
    // TODO: refactor into a config struct
    pub fn new(
        workers: u32,
        listener: async_channel::Receiver<ManagerStream>,
    ) -> Result<Self, ManagerError> {
        let (job_sender, job_receiver) = async_channel::unbounded();
        let (state_sender, state_receiver) = async_channel::unbounded();
        for _ in 0..workers {
            tokio::spawn(DownloadWorker::new(job_receiver.clone(), state_sender.clone()).work());
        }
        tokio::spawn(StateDaemon::new(state_receiver, "/tmp/test.db")?.work());
        Ok(ManagerDaemon {
            server: listener,
            job_sender,
            state_client: StateClient::new(state_sender),
            cancel_channels: HashMap::new()
        })
    }
    pub async fn serve(mut self) -> Result<(), ManagerError> {
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
        }
    }
    async fn handle(&mut self, api: &mut ManagerStream) -> Result<Message, ManagerError> {
        let cmd = api.read().await?;
        match cmd {
            Message::Add(c) => {
                println!("adding {}", c.url);
                Ok(self.add(&c.url).await?)
            }
            Message::Info(c) => {
                println!("querying {}", c.name);
                Ok(self.info(&c.name).await?)
            }
            Message::Cancel(c) => {
                println!("cancelling");
                Ok(self.cancel(&c.name).await?)
            }
            Message::List(_) => {
                println!("listing");
                Ok(self.list().await?)
            }
            _ => {
                println!("thanks for the acknowledgment, don't expect it though");
                Ok(Message::Error(ManagerError {
                    kind: InvalidMessage,
                    msg: format!("unexpected command {:?}", cmd).into(),
                }))
            }
        }
    }
    async fn add(&mut self, url: &str) -> Result<Message, ManagerError> {
        print!("url is: {}", url);
        let u = Url::parse(url)?;
        let segments = u.path_segments();
        let mut name = "unnamed";
        if let Some(segments) = segments {
            name = segments.last().unwrap_or("unnamed");
        }
        let (tx, rx) = watch::channel(false);
        let file_path = Path::new("/tmp/Downloads").join(name);
        let job = DownloadJob {
            name: name.into(),
            // TODO: make configurable
            file_path: file_path.clone(),
            url: url.to_string(),
            cancel_channel: rx
        };
        self.state_client
            .update(JobState {
                name: name.into(),
                url: url.into(),
                path: file_path.to_str().unwrap_or("invalid path, shouldn't happen").into(), // TODO: unsafe wrap?
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
    async fn cancel(&mut self, name: &str) -> Result<Message, ManagerError> {
        let ch = self.cancel_channels.remove(name).ok_or(ManagerError{
            kind: DownloadJobNotFound,
            msg: format!("couldn't find {}", name)
        })?;
        // if it's done, nothing will be done
        // if it's pending, a worker will pick it up, know it's cancelled and skip it
        // if it's active, the worker will cancel after downloading the current chunk
        ch.send(true)?;
        Ok(Message::Ack(AckCommand))
    }
}
