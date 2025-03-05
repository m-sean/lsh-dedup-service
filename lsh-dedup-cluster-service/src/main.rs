mod dedup;
mod lsh;
mod util;

use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use lazy_static::lazy_static;
use lsh_dedup_service::dto::DedupConfig;
use lsh_dedup_service::error::ServiceError;
use lsh_dedup_service::response::make_response_payload;
use lsh_dedup_service::util::get_region;
use rusoto_core::{Client, Region};
use rusoto_s3::S3Client;
use serde_json::Value;

lazy_static! {
    // AWS Region
    static ref REGION: Region = get_region().unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(process)).await?;
    Ok(())
}

async fn process(event: LambdaEvent<DedupConfig>) -> Result<Value, Error> {
    let (config, _context) = event.into_parts();
    let result = dedup(config).await;
    make_response_payload(result)
}

async fn dedup(config: DedupConfig) -> Result<Value, ServiceError> {
    let start = std::time::Instant::now();
    let client = S3Client::new_with_client(Client::shared(), REGION.clone());
    let records = util::pull_data_file(&client, &config.data).await?;
    println!(
        "File downloaded in {:.4} secs",
        (std::time::Instant::now() - start).as_secs_f64()
    );
    let start = std::time::Instant::now();
    let lsh = lsh::MinHashLSH::new(&records, config.num_perm, config.num_bands);
    println!(
        "Hashed records in {:.4} secs",
        (std::time::Instant::now() - start).as_secs_f64()
    );
    let dedup_table = dedup::DeduplicationTable::new(lsh, Some(config.threshold));
    println!(
        "Dedupe completed in {:.4} secs",
        (std::time::Instant::now() - start).as_secs_f64()
    );
    // let results: Vec<RecordResult> = dedup_table
    //     .grouped_ids()
    //     .into_iter()
    //     .enumerate()
    //     .flat_map(|(idx, group)| {
    //         let cluster_id = format!("{idx}-{}", group.len());
    //         group.into_iter().map(move |id| RecordResult {
    //             id: id.to_string(),
    //             cluster_id: cluster_id.clone(),
    //         })
    //     })
    //     .collect();
    util::push_result_file(&client, config.data.bucket, config.data.key, dedup_table).await
}
