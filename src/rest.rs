use crate::daemon1::client::DaemonClient;
use crate::daemon1::types::{AckCommand, InfoResponse, ListResponse};
use crate::err::ManagerError;
use crate::err::ManagerErrorKind;
use rocket::http::{ContentType, Status};
use rocket::response;
use rocket::response::{Responder, Response};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::Request;
use rocket::State;
use std::fmt;
#[derive(Serialize, Deserialize)]
pub struct Add {
    url: String,
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
    json: Json<T>,
    status: Status,
}

impl<'r, T: Serialize> Responder<'r, 'r> for ApiResponse<T> {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        Response::build_from(self.json.respond_to(&req)?)
            .status(self.status)
            .header(ContentType::JSON)
            .ok()
    }
}

// TODO: error handling
#[get("/")]
pub async fn list(
    state: &State<DaemonClient>,
) -> Result<ApiResponse<ListResponse>, ApiResponse<Error>> {
    match state.list().await {
        Ok(v) => Ok(ApiResponse {
            json: Json(v),
            status: Status::Ok,
        }),
        Err(e) => Err(ApiResponse {
            json: Json(e.into()),
            status: Status::InternalServerError,
        }),
    }
}

#[get("/<name>")]
pub async fn info(
    state: &State<DaemonClient>,
    name: &str,
) -> Result<ApiResponse<InfoResponse>, ApiResponse<Error>> {
    match state.info(name).await {
        Ok(v) => Ok(ApiResponse {
            json: Json(v),
            status: Status::Ok,
        }),
        Err(e) => {
            let code = match &e.kind {
                ManagerErrorKind::DownloadJobNotFound => Status::NotFound,
                _ => Status::InternalServerError,
            };
            Err(ApiResponse {
                json: Json(e.into()),
                status: code,
            })
        }
    }
}

#[delete("/<name>")]
pub async fn cancel(
    state: &State<DaemonClient>,
    name: &str,
) -> Result<ApiResponse<AckCommand>, ApiResponse<Error>> {
    match state.cancel(name).await {
        Ok(v) => Ok(ApiResponse {
            json: Json(v),
            status: Status::Ok,
        }),
        Err(e) => {
            let code = match &e.kind {
                ManagerErrorKind::DownloadJobNotFound => Status::NotFound,
                _ => Status::InternalServerError,
            };
            Err(ApiResponse {
                json: Json(e.into()),
                status: code,
            })
        }
    }
}

#[post("/", format = "application/json", data = "<msg>")]
pub async fn add(
    state: &State<DaemonClient>,
    msg: Json<Add>,
) -> Result<ApiResponse<AckCommand>, ApiResponse<Error>> {
    match state.add(&msg.url).await {
        Ok(v) => Ok(ApiResponse {
            json: Json(v),
            status: Status::Created,
        }),
        Err(e) => {
            let code = match &e.kind {
                ManagerErrorKind::InvalidAddress => Status::BadRequest,
                _ => Status::InternalServerError,
            };
            Err(ApiResponse {
                json: Json(e.into()),
                status: code,
            })
        }
    }
}

#[catch(404)]
pub fn not_found(_: &Request) -> ApiResponse<Error> {
    ApiResponse {
        json: Json("page not found".into()),
        status: Status::NotFound,
    }
}

#[catch(500)]
pub fn internal_server_error(_: &Request) -> ApiResponse<Error> {
    ApiResponse {
        json: Json("page not found".into()),
        status: Status::InternalServerError,
    }
}
