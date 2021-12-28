use crate::err::ManagerError;
use rocket::http::{ContentType, Status};
use rocket::response::{Responder, Response};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{response, Request};
use std::fmt;

#[derive(Serialize, Deserialize)]
pub struct Add {
    pub url: String,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Cancel {
    pub forget: bool,
    pub delete: bool,
}

#[derive(Serialize, Deserialize)]
pub enum Error {
    Error(String),
}

impl From<ManagerError> for Error {
    fn from(e: ManagerError) -> Self {
        Error::Error(e.to_string())
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Error::Error(e.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let Error::Error(e) = self;
        write!(f, "{}", e)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub json: Json<T>,
    pub status: Status,
}

impl<'r, T: Serialize> Responder<'r, 'r> for ApiResponse<T> {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        Response::build_from(self.json.respond_to(req)?)
            .status(self.status)
            .header(ContentType::JSON)
            .ok()
    }
}
