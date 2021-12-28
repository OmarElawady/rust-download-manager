use std::fmt;

#[derive(Clone, Debug)]
pub struct JobInfo {
    pub name: String,
    pub url: String,
    pub path: String,
    pub downloaded: u64,
    pub total: u64,
    pub state: State,
    pub msg: String,
}

#[derive(PartialEq, Clone, Debug)]
pub enum State {
    Active,
    Pending,
    Cancelled,
    Failed,
    Done,
    Unknown,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                State::Active => "Active",
                State::Pending => "Pending",
                State::Failed => "Failed",
                State::Cancelled => "Cancelled",
                State::Done => "Done",
                State::Unknown => "Unknown",
            }
        )
    }
}
