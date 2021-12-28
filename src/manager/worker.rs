use super::types::CancelInfo;
use super::types::DownloadJob;
use crate::jobs::client::StateClient;
use crate::jobs::types::JobMessage;
use crate::types::{JobInfo, State};
use reqwest::header::RANGE;
use reqwest::StatusCode;
use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt;
use tokio::fs::{metadata, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;

pub struct DownloadWorker {
    job_receiver: async_channel::Receiver<DownloadJob>,
    state_client: StateClient,
}

impl DownloadWorker {
    pub fn new(
        job_receiver: async_channel::Receiver<DownloadJob>,
        state_sender: async_channel::Sender<JobMessage>,
    ) -> Self {
        DownloadWorker {
            job_receiver,
            state_client: StateClient::new(state_sender),
        }
    }

    pub async fn work(self) {
        while let Ok(job) = self.job_receiver.recv().await {
            let mut state = JobInfo {
                name: job.name.clone(),
                url: job.url.clone(),
                path: job
                    .file_path
                    .to_str()
                    .unwrap_or("invalid path, shouldn't happen")
                    .into(),
                downloaded: 0,
                total: 0,
                state: State::Active,
                msg: "".into(),
            };

            if let Some(err) = self.download(&job, &mut state).await {
                state.state = State::Failed;
                state.msg = err;
                self.update_state(state, &job.cancel_channel).await;
            }
        }
        println!("worker died!!!");
    }
    async fn update_state(&self, state: JobInfo, cancelled: &watch::Receiver<CancelInfo>) {
        if cancelled.borrow().cancel {
            return;
        }
        let res = self.state_client.update(state).await;
        if let Err(e) = res {
            println!("failed to update state {}", e)
        }
    }
    async fn check_partial_content_support(url: String) -> Result<bool, reqwest::Error> {
        // got empty response from a server while using head
        let req = reqwest::Client::new()
            .get(url)
            .header(RANGE, "bytes=0-0")
            .send()
            .await?;
        Ok(req.status() == StatusCode::PARTIAL_CONTENT)
    }
    // returns an error message if something bad happened
    async fn download(&self, job: &DownloadJob, state: &mut JobInfo) -> Option<String> {
        if job.cancel_channel.borrow().cancel {
            return None;
        }
        let mut req = reqwest::Client::new().get(job.url.clone());

        let file_metadata = metadata(&job.file_path).await;
        match &file_metadata {
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    // new download
                } else {
                    return Some(format!("couldn't stat download path {}", e.to_string()));
                }
            }
            Ok(v) => match Self::check_partial_content_support(job.url.clone()).await {
                Err(e) => {
                    return Some(format!(
                        "couldn't check url support for partial downloads {}",
                        e
                    ));
                }
                Ok(supported) => {
                    if supported {
                        println!("adding range header {}", format!("bytes={}-", v.size()));
                        req = req.header(RANGE, format!("bytes={}-", v.size()));
                        state.downloaded = v.size();
                    } else {
                        return Some("remote url doesn't support partial downloads".to_string());
                    }
                }
            },
        }
        self.update_state(state.clone(), &job.cancel_channel).await;
        let res = req.send().await;

        if let Err(e) = res {
            return Some(e.to_string());
        }
        let mut res = res.unwrap();
        let headers = res.headers();
        if let Some(len_str) = headers.get("Content-Length") {
            let len = len_str.to_str().unwrap_or("0").parse().unwrap_or(0);
            state.total = len + file_metadata.map(|v| v.size()).unwrap_or(0);
            self.update_state(state.clone(), &job.cancel_channel).await;
        }
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&job.file_path)
            .await;
        if let Err(e) = file {
            return Some(format!("failed to create file: {}", e.to_string()));
        }
        let mut file = file.unwrap();
        loop {
            if job.cancel_channel.borrow().cancel {
                if job.cancel_channel.borrow().delete {
                    if let Err(e) = std::fs::remove_file(job.file_path.clone()) {
                        println!("failed to remove download {}", e)
                    }
                }
                return None;
            }
            let chunk = res.chunk().await;
            match chunk {
                Ok(chunk) => match chunk {
                    Some(chunk) => match file.write_all(&chunk).await {
                        Ok(_) => {
                            state.downloaded += chunk.len() as u64;
                            self.update_state(state.clone(), &job.cancel_channel).await;
                        }
                        Err(e) => {
                            return Some(format!("failed to download chunk: {}", e.to_string()));
                        }
                    },
                    None => {
                        state.state = State::Done;
                        self.update_state(state.clone(), &job.cancel_channel).await;
                        return None;
                    }
                },
                Err(e) => {
                    return Some(format!("failed to download chunk: {}", e.to_string()));
                }
            }
        }
    }
}
