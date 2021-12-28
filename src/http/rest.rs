use super::types::{Add, ApiResponse, Cancel, Error};
use crate::err::ManagerErrorKind;
use crate::manager::client::ManagerClient;
use crate::manager::types::{AckCommand, InfoResponse, ListResponse};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Request, State};

#[get("/")]
pub async fn list(
    state: &State<ManagerClient>,
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
    state: &State<ManagerClient>,
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

#[delete("/<name>", format = "application/json", data = "<msg>")]
pub async fn cancel(
    state: &State<ManagerClient>,
    name: &str,
    msg: Json<Cancel>,
) -> Result<ApiResponse<AckCommand>, ApiResponse<Error>> {
    match state.cancel(name, msg.forget, msg.delete).await {
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
    state: &State<ManagerClient>,
    msg: Json<Add>,
) -> Result<ApiResponse<AckCommand>, ApiResponse<Error>> {
    match state.add(&msg.url, msg.name.as_deref()).await {
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
