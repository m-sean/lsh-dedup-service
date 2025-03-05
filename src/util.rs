use crate::error::ServiceError;
use crate::response::Status;
use futures::stream::TryStreamExt;
use rusoto_core::Region;
use rusoto_s3::{GetObjectRequest, PutObjectRequest, S3Client, S3};
use std::env;
use std::str::FromStr;

pub fn get_region() -> Result<Region, ServiceError> {
    match env::var("REGION") {
        Ok(val) => Region::from_str(val.as_str()).map_err(|_| ServiceError {
            msg: format!("Unable to parse region {}", val),
            status: Status::InternalServerError,
        }),
        _ => Err(ServiceError::internal_server_error(
            "Environment variable 'REGION' not found",
        )),
    }
}

pub fn get_env_var(name: &str) -> Result<String, ServiceError> {
    env::var(name).map_err(|_| {
        ServiceError::internal_server_error(&format!("Environment variable '{}' not found", name))
    })
}

pub async fn download_object_from_s3(
    client: &S3Client,
    bucket: String,
    key: String,
) -> Result<Vec<u8>, ServiceError> {
    let request = GetObjectRequest {
        bucket,
        key,
        ..Default::default()
    };
    let mut object = client
        .get_object(request)
        .await
        .map_err(ServiceError::internal_server_error)?;
    let body = object
        .body
        .take()
        .ok_or(ServiceError::internal_server_error(
            "Unable to extract body",
        ))?;
    body.map_ok(|b| b.to_vec())
        .try_concat()
        .await
        .map_err(ServiceError::internal_server_error)
}

pub async fn upload_object_to_s3(
    client: &S3Client,
    object: Vec<u8>,
    bucket: String,
    key: String,
) -> Result<(), ServiceError> {
    let request = PutObjectRequest {
        bucket,
        key,
        body: Some(object.into()),
        ..Default::default()
    };
    client
        .put_object(request)
        .await
        .map(|_| ())
        .map_err(ServiceError::internal_server_error)
}
