use super::db::Database;
use crate::err::ManagerError;
use crate::jobs::types::JobMessage;

pub struct StateDaemon {
    state_receiver: async_channel::Receiver<JobMessage>,
    db: Database,
}

impl StateDaemon {
    pub fn new(
        state_receiver: async_channel::Receiver<JobMessage>,
        db: &str,
    ) -> Result<Self, ManagerError> {
        let db = Database::new(db)?;
        Ok(StateDaemon { state_receiver, db })
    }
    pub async fn work(self) {
        while let Ok(state) = self.state_receiver.recv().await {
            match state {
                JobMessage::Update(msg) => {
                    let res = self.db.update_state(msg.job_state);
                    let _ = match res {
                        Err(e) => msg.response_channel.send(JobMessage::Error(e)).await,
                        Ok(_) => msg.response_channel.send(JobMessage::Ack).await,
                    };
                }
                JobMessage::UpdateState(msg) => {
                    let res = self.db.update_job_state(&msg.name, msg.state);
                    let _ = match res {
                        Err(e) => msg.response_channel.send(JobMessage::Error(e)).await,
                        Ok(_) => msg.response_channel.send(JobMessage::Ack).await,
                    };
                }
                JobMessage::Delete(msg) => {
                    let res = self.db.delete_job(&msg.name);
                    let _ = match res {
                        Err(e) => msg.response_channel.send(JobMessage::Error(e)).await,
                        Ok(_) => msg.response_channel.send(JobMessage::Ack).await,
                    };
                }
                JobMessage::Get(msg) => {
                    let res = self.db.get_job(&msg.name);
                    let _ = match res {
                        Err(e) => msg.response_channel.send(JobMessage::Error(e)).await,
                        Ok(v) => msg.response_channel.send(JobMessage::GetResponse(v)).await,
                    };
                }
                JobMessage::List(msg) => {
                    let res = self.db.list_jobs();
                    let _ = match res {
                        Err(e) => msg.response_channel.send(JobMessage::Error(e)).await,
                        Ok(v) => msg.response_channel.send(JobMessage::ListResponse(v)).await,
                    };
                }
                _ => {
                    println!("state daemon got an unexpected message {:?}", state)
                }
            }
        }
        println!("state daemon exited, oh noooo")
    }
}
