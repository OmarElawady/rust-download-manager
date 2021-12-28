use crate::err::ManagerError;
use crate::types::JobState;

#[derive(Debug)]
pub struct StateUpdateMessage {
    pub job_state: JobState,
    pub response_channel: async_channel::Sender<StateMessage>,
}
#[derive(Debug)]
pub struct StateGetMessage {
    pub name: String,
    pub response_channel: async_channel::Sender<StateMessage>,
}
#[derive(Debug)]
pub struct StateListMessage {
    pub response_channel: async_channel::Sender<StateMessage>,
}

#[derive(Debug)]
pub enum StateMessage {
    Update(StateUpdateMessage),
    Get(StateGetMessage),
    List(StateListMessage),
    GetResponse(JobState),
    ListResponse(Vec<JobState>),
    Ack,
    Error(ManagerError),
}
