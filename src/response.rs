use serde::{de, Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;

use crate::error::ServiceError;

#[derive(Debug, Clone)]
pub enum Status {
    Ok,
    Accepted,
    BadRequest,
    InternalServerError,
    GatewayTimeout,
}

impl Status {
    fn code(&self) -> u16 {
        match self {
            Status::Ok => 200,
            Status::Accepted => 202,
            Status::BadRequest => 400,
            Status::InternalServerError => 500,
            Status::GatewayTimeout => 504,
        }
    }
}

impl Serialize for Status {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u16(self.code())
    }
}

struct StatusCodeVisitor;

impl<'de> de::Visitor<'de> for StatusCodeVisitor {
    type Value = Status;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Status")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v {
            200 => Ok(Status::Ok),
            202 => Ok(Status::Accepted),
            400 => Ok(Status::BadRequest),
            500 => Ok(Status::InternalServerError),
            504 => Ok(Status::GatewayTimeout),
            value => Err(de::Error::custom(value.to_string())),
        }
    }
}

impl<'de> de::Deserialize<'de> for Status {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_u16(StatusCodeVisitor)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ResponsePayload {
    pub status_code: Status,
    pub headers: Value,
    pub body: Value,
}

pub fn make_response_payload(
    result: Result<Value, ServiceError>,
) -> Result<Value, lambda_runtime::Error> {
    let headers = json!({
        "Content-Type": "application/json",
        "Access-Control-Allow-Origin": "*"
    });
    let response_payload = match result {
        Err(err) => ResponsePayload {
            status_code: err.status,
            headers,
            body: Value::String(err.msg),
        },
        Ok(body) => ResponsePayload {
            status_code: Status::Ok,
            headers,
            body,
        },
    };
    serde_json::to_value(response_payload).map_err(lambda_runtime::Error::from)
}
