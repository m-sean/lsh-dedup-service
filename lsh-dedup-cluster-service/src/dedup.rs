use crate::lsh::MinHashLSH;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

///
/// Wraps two mappings to determine duplicate clusters based on the results
/// from querying the MinhashLSH.
///
pub struct DeduplicationTable<'a> {
    /// Mapping of duplicate group id to record ids
    duplicate_groups: HashMap<usize, HashSet<&'a str>>,
    /// Reverse lookup to identify the group id of a record id
    doc_lookup: HashMap<&'a str, usize>,
}

impl<'a> DeduplicationTable<'a> {
    ///
    /// Constructs an DeduplicationTable instance using an existing MinHashLSH for querying and clustering.
    ///
    /// ## Arguments
    ///
    /// * `lsh` - A MinHashLSH to use for querying record similarity.
    /// * `threshold` - The jaccard similarity threshold (inclusive) to filter query results (optional).
    ///
    pub fn new(lsh: MinHashLSH<'a>, threshold: Option<f64>) -> Self {
        let query_results: Vec<(&str, Vec<&str>)> = lsh
            .minhash_map
            .par_iter()
            .map(|(&id, minhash)| (id, lsh.query(minhash, threshold)))
            .collect();
        Self::from_query_results(query_results)
    }

    ///
    /// Outputs the list of clustered record ids found to be similar enough to form distinct groups.
    ///
    pub fn grouped_ids(&self) -> Vec<Vec<&'a str>> {
        self.duplicate_groups
            .values()
            .map(|set| set.into_iter().map(|&s| s).collect())
            .collect()
    }

    fn init() -> Self {
        Self {
            duplicate_groups: HashMap::new(),
            doc_lookup: HashMap::new(),
        }
    }

    fn from_query_results(query_results: Vec<(&'a str, Vec<&'a str>)>) -> Self {
        let mut document_clusters = Self::init();
        for (query_doc_id, similar_documents) in query_results {
            let cluster_id = document_clusters
                .check_cluster_id(query_doc_id)
                .unwrap_or(document_clusters.new_id());
            let mut current_cluster = HashSet::new();
            for similar_doc_id in similar_documents {
                if let Some(prev_cluster_id) = document_clusters.check_cluster_id(similar_doc_id) {
                    if prev_cluster_id != cluster_id {
                        let reassignment = document_clusters.remove(prev_cluster_id);
                        current_cluster.extend(reassignment);
                    }
                } else {
                    if !current_cluster.contains(similar_doc_id) {
                        current_cluster.insert(similar_doc_id);
                    }
                }
            }
            document_clusters.update(cluster_id, current_cluster);
        }
        document_clusters
    }

    fn update(&mut self, cluster_id: usize, doc_set: HashSet<&'a str>) {
        for doc_id in doc_set {
            self.add(cluster_id, doc_id);
        }
    }

    fn add(&mut self, cluster_id: usize, doc_id: &'a str) {
        self.duplicate_groups
            .entry(cluster_id)
            .or_insert_with(HashSet::new)
            .insert(doc_id);
        self.doc_lookup.insert(doc_id, cluster_id);
    }

    fn check_cluster_id(&mut self, doc_id: &'a str) -> Option<usize> {
        self.doc_lookup.get(doc_id).copied()
    }

    fn new_id(&self) -> usize {
        self.duplicate_groups.keys().max().map_or(0, |&v| v + 1)
    }

    fn remove(&mut self, cluster_id: usize) -> HashSet<&'a str> {
        let set = self
            .duplicate_groups
            .remove(&cluster_id)
            .expect(&format!("Cluster {cluster_id} doesn't exist"));
        for k in set.iter() {
            self.doc_lookup.remove(k);
        }
        set
    }
}
