use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct DataFile {
    pub bucket: String,
    pub key: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DedupConfig {
    pub data: DataFile,
    pub num_perm: usize,
    pub num_bands: usize,
    pub threshold: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Record {
    pub id: String,
    pub text: String,
}

pub struct RecordResult {
    pub id: String,
    pub cluster_id: String,
}
