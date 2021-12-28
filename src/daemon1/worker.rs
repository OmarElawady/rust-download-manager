use super::types::DownloadJob;
use crate::state::client::StateClient;
use crate::state::types::StateMessage;
use crate::types::{JobState, State};
use async_channel;
use reqwest;
use std::fs::File;
use std::io::prelude::*;

pub struct DownloadWorker {
    job_receiver: async_channel::Receiver<DownloadJob>,
    state_client: StateClient,
}

impl DownloadWorker {
    pub fn new(
        job_receiver: async_channel::Receiver<DownloadJob>,
        state_sender: async_channel::Sender<StateMessage>,
    ) -> Self {
        DownloadWorker {
            job_receiver,
            state_client: StateClient::new(state_sender),
        }
    }

    pub async fn work(self) {
        while let Ok(msg) = self.job_receiver.recv().await {
            self.download(&msg).await
        }
    }
    async fn update_state(&self, state: JobState) {
        let res = self.state_client.update(state).await;
        if let Err(e) = res {
            println!("failed to update state {}", e)
        }
    }
    async fn download(&self, job: &DownloadJob) {
        let req = reqwest::get(job.url.clone()).await;
        if let Err(e) = req {
            // ignore error
            self.update_state(JobState {
                name: job.name.clone(),
                url: job.url.clone(),
                path: job.file_path.to_str().unwrap_or("invalid path, shouldn't happen").into(),
                downloaded: 0,
                total: 0,
                state: State::Failed,
                msg: e.to_string(),
            })
            .await;
            return;
        }
        let mut state = JobState {
            name: job.name.clone(),
            url: job.url.clone(),
            path: job.file_path.to_str().unwrap_or("invalid path, shouldn't happen").into(),
            downloaded: 0,
            total: 0,
            state: State::Active,
            msg: "".into(),
        };
        if *job.cancel_channel.borrow() == true {
            state.state = State::Cancelled;
            self.update_state(state.clone()).await;
            return
        }
        self.update_state(state.clone()).await;
        let mut req = req.unwrap(); // safe unwrap
        let headers = req.headers();
        if let Some(len_str) = headers.get("Content-Length") {
            let len = len_str.to_str().unwrap_or("0").parse().unwrap_or(0);
            state.total = len;
            self.update_state(state.clone()).await;
        }
        let file = File::create(job.file_path.clone());
        if let Err(e) = file {
            state.msg = format!("failed to create file: {}", e.to_string());
            state.state = State::Failed;
            self.update_state(state.clone()).await;
            return;
        }
        let mut file = file.unwrap();
        loop {
            if *job.cancel_channel.borrow() == true {
                state.state = State::Cancelled;
                self.update_state(state.clone()).await;
                return
            }
            let chunk = req.chunk().await;
            // TODO: there must be a way to chain this
            match chunk {
                Ok(chunk) => match chunk {
                    Some(chunk) => match file.write_all(&chunk) {
                        Ok(_) => {
                            state.downloaded += chunk.len() as u64;
                            self.update_state(state.clone()).await;
                        }
                        Err(e) => {
                            state.msg =
                                format!("failed to write downloaded chunk: {}", e.to_string());
                            state.state = State::Failed;
                            self.update_state(state.clone()).await;
                            return;
                        }
                    },
                    None => {
                        state.state = State::Done;
                        self.update_state(state.clone()).await;
                        return;
                    }
                },
                Err(e) => {
                    state.msg = format!("failed to download chunk: {}", e.to_string());
                    state.state = State::Failed;
                    self.update_state(state.clone()).await;
                    return;
                }
            }
        }
    }
}
