use crate::response::Status;
use serde::{Deserialize, Serialize};
use serde_json;
use std::error;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceError {
    pub msg: String,
    pub status: Status,
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let json = serde_json::to_string_pretty(&self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json)
    }
}

impl error::Error for ServiceError {}

impl ServiceError {
    pub fn bad_request<T: std::fmt::Display>(msg: T) -> ServiceError {
        ServiceError {
            msg: msg.to_string(),
            status: Status::BadRequest,
        }
    }

    pub fn internal_server_error<T: std::fmt::Display>(msg: T) -> ServiceError {
        ServiceError {
            msg: msg.to_string(),
            status: Status::InternalServerError,
        }
    }
}
