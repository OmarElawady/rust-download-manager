use super::db::Database;
use crate::err::ManagerError;
use crate::state::types::StateMessage;

pub struct StateDaemon {
    state_receiver: async_channel::Receiver<StateMessage>,
    db: Database,
}

impl StateDaemon {
    pub fn new(
        state_receiver: async_channel::Receiver<StateMessage>,
        db: &str,
    ) -> Result<Self, ManagerError> {
        let db = Database::new(db)?;
        Ok(StateDaemon { state_receiver, db })
    }
    pub async fn work(self) {
        while let Ok(state) = self.state_receiver.recv().await {
            match state {
                StateMessage::Update(msg) => {
                    let res = self.db.update_state(msg.job_state);
                    let _ = match res {
                        Err(e) => msg.response_channel.send(StateMessage::Error(e)).await,
                        Ok(_) => msg.response_channel.send(StateMessage::Ack).await,
                    };
                }
                StateMessage::Get(msg) => {
                    let res = self.db.get_state(&msg.name);
                    let _ = match res {
                        Err(e) => msg.response_channel.send(StateMessage::Error(e)).await,
                        Ok(v) => {
                            msg.response_channel
                                .send(StateMessage::GetResponse(v))
                                .await
                        }
                    };
                }
                StateMessage::List(msg) => {
                    let res = self.db.list_states();
                    let _ = match res {
                        Err(e) => msg.response_channel.send(StateMessage::Error(e)).await,
                        Ok(v) => {
                            msg.response_channel
                                .send(StateMessage::ListResponse(v))
                                .await
                        }
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
