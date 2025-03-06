use lsh_dedup_service::dto::DedupConfig;
use lsh_dedup_service::response::{make_response_payload, ResponsePayload, Status};
use lsh_dedup_service::{error::ServiceError, util};

use bytes::Bytes;
use lambda_runtime::{run, service_fn, Context, Error, LambdaEvent};
use lazy_static::lazy_static;
use reqwest;
use rusoto_core::{Client, Region};
use rusoto_kms::{DecryptRequest, Kms, KmsClient};
use serde_json::{json, Value};
use std::collections::HashMap;

lazy_static! {
    // AWS Region
    static ref REGION: Region = util::get_region().unwrap();
    // Callback Endpoint
    static ref ENDPOINT: String = util::get_env_var("ENDPOINT").unwrap();
    // Encrypted Api Key
    static ref API_KEY: String = util::get_env_var("API_KEY").unwrap();
    // Symmetric encryption key ID stored in AWS KMS
    static ref KEY_ID: String = util::get_env_var("KEY_ID").unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(process)).await?;
    Ok(())
}

async fn process(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let (payload, context) = event.into_parts();
    let result = callback(payload, &context).await;
    make_response_payload(result)
}

async fn callback(payload: Value, context: &Context) -> Result<Value, ServiceError> {
    let body = extract(payload)?;
    let client = reqwest::Client::builder()
        .build()
        .map_err(ServiceError::internal_server_error)?;
    let key = decrypt_api_key(function_name(context)?).await?;
    client
        .post(ENDPOINT.as_str())
        .header("X-API-KEY", key.as_str())
        .header("CONTENT-TYPE", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(ServiceError::internal_server_error)?
        .error_for_status()
        .map_err(ServiceError::internal_server_error)?;
    Ok(json!({}))
}

fn extract(payload: Value) -> Result<Value, ServiceError> {
    let resp_payload = payload
        .get("responsePayload")
        .ok_or_else(|| ise("No 'responsePayload' object found"))?
        .to_owned();
    let (status, body) = match serde_json::from_value(resp_payload.clone()) {
        Ok(ResponsePayload { status_code, body, .. }) => match status_code {
            Status::Ok => (status_code, body),
            _ => {
                let config: DedupConfig = payload
                    .get("requestPayload")
                    .map(|v| {
                        serde_json::from_value(v.to_owned())
                            .map_err(ServiceError::internal_server_error)
                    })
                    .ok_or_else(|| ise("No 'requestPayload' object found"))??;
                (status_code, json!({ "message": body, "config": config }))
            }
        },
        Err(_) => {
            let msg = resp_payload
                .get("errorMessage")
                .ok_or_else(|| ise("No 'errorMessage' field found"))?
                .as_str()
                .ok_or_else(|| ise("Unable to parse error message"))?;
            let status = if msg.contains("timed out") {
                Status::GatewayTimeout
            } else {
                Status::InternalServerError
            };
            let req_payload = payload
                .get("requestPayload")
                .ok_or_else(|| ise("No 'requestPayload' object found"))?
                .to_owned();
            let config: DedupConfig =
                serde_json::from_value(req_payload).map_err(ServiceError::internal_server_error)?;
            (status, json!({ "message": msg, "config": config }))
        }
    };

    Ok(json!({
        "statusCode": status,
        "body": body,
    }))
}

fn ise(msg: &str) -> ServiceError {
    ServiceError::internal_server_error(msg)
}

async fn decrypt_api_key(function_name: &str) -> Result<String, ServiceError> {
    let client = KmsClient::new_with_client(Client::shared(), REGION.clone());
    let context = HashMap::from([(
        String::from("LambdaFunctionName"),
        String::from(function_name),
    )]);
    let ciphertext = base64::decode(API_KEY.as_bytes()).map_err(|err| ise(&err.to_string()))?;
    let request = DecryptRequest {
        ciphertext_blob: Bytes::from(ciphertext),
        key_id: Some(KEY_ID.clone()),
        encryption_context: Some(context),
        ..Default::default()
    };
    let response = client
        .decrypt(request)
        .await
        .map_err(|err| ise(&err.to_string()))?;
    if let Some(bytes) = response.plaintext {
        String::from_utf8(bytes.into_iter().collect()).map_err(|err| ise(&err.to_string()))
    } else {
        Err(ServiceError::internal_server_error(
            "Unable to decode api key",
        ))
    }
}

fn function_name(context: &Context) -> Result<&str, ServiceError> {
    let splits: Vec<&str> = context.invoked_function_arn.split(':').collect();
    splits
        .last()
        .map(|&s| s)
        .ok_or_else(|| ise("Unable to extract feature name"))
}
