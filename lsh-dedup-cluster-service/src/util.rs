use crate::dedup::DeduplicationTable;
use csv::{Reader, Writer};
use lsh_dedup_service::dto::{DataFile, Record};
use lsh_dedup_service::error::ServiceError;
use lsh_dedup_service::util::{download_object_from_s3, upload_object_to_s3};
use rusoto_s3::S3Client;
use serde_json::{json, Value};

pub async fn pull_data_file(
    client: &S3Client,
    data: &DataFile,
) -> Result<Vec<Record>, ServiceError> {
    let bytes = download_object_from_s3(client, data.bucket.clone(), data.key.clone()).await?;
    let mut reader = Reader::from_reader(bytes.as_slice());
    let headers = reader
        .headers()
        .map_err(ServiceError::internal_server_error)?
        .clone();
    reader
        .records()
        .into_iter()
        .map(|record| match record {
            Ok(rec) => rec.deserialize(Some(&headers)).map_err(|_| {
                ServiceError::bad_request(String::from("file must contain columns 'id' and 'text'"))
            }),
            Err(err) => Err(ServiceError::internal_server_error(err)),
        })
        .collect()
}

pub async fn push_result_file<'a>(
    client: &S3Client,
    bucket: String,
    key: String,
    dedup_table: DeduplicationTable<'a>,
) -> Result<Value, ServiceError> {
    let mut writer = Writer::from_writer(vec![]);
    let record_map = dedup_table
        .grouped_ids()
        .into_iter()
        .enumerate()
        .flat_map(|(idx, group)| {
            let cluster_id = format!("{idx}-{}", group.len());
            group
                .into_iter()
                .map(move |id| (id.to_string(), cluster_id.clone()))
        });
    for (rec_id, cluster_id) in record_map {
        writer
            .write_record(&[rec_id, cluster_id])
            .map_err(ServiceError::internal_server_error)?;
    }
    let object = writer
        .into_inner()
        .map_err(ServiceError::internal_server_error)?;
    let output_bucket = bucket.replace("/input", "/output");
    upload_object_to_s3(client, object, output_bucket.clone(), key.clone()).await?;
    Ok(json!({ "bucket": output_bucket, "key": key }))
}
