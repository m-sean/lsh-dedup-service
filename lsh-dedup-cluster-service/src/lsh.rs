use lsh_dedup_service::dto::Record;
use rand::prelude::*;
use rayon::prelude::*;
use rustc_hash::FxHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct MinHash {
    pub hash_values: Vec<u32>,
    num_perm: usize,
}

impl MinHash {
    fn new(items: Vec<&str>, permutations: &Vec<(u64, u64)>) -> Self {
        let num_perm = permutations.len();
        let mut hash_values = vec![u32::MAX; num_perm];
        for item in items {
            let item_hash = calculate_hash(&item);
            for (i, &(a, b)) in permutations.iter().enumerate() {
                let hash = permute_hash(item_hash, a, b);
                hash_values[i] = hash_values[i].min(hash);
            }
        }
        MinHash {
            hash_values,
            num_perm,
        }
    }

    pub fn jaccard_similarity(&self, other: &MinHash) -> f64 {
        let equal_count = self
            .hash_values
            .par_iter()
            .zip(&other.hash_values)
            .filter(|&(&a, &b)| a == b)
            .count();
        equal_count as f64 / self.num_perm as f64
    }
}

#[derive(Clone)]
/// Locality-Sensitive Hashing using MinHash for efficient similarity search.
pub struct MinHashLSH<'a> {
    /// A table for looking up full minhashes for jaccard similarity thresholding
    pub minhash_map: HashMap<&'a str, MinHash>,
    /// Number of times to split the hash singature (number of banded hash tables)
    band_size: usize,
    /// Banded hash tables used to find candidates for similarity
    hash_tables: Vec<HashMap<u64, Vec<&'a str>>>,
}

impl<'a> MinHashLSH<'a> {
    /// Creates a new MinHashLSH instance.
    ///
    /// ## Arguments
    ///
    /// * `records` - The records to dedupe.
    /// * `num_perm` - Number of permutations to use in the MinHash algorithm.
    /// * `num_bands` - Number of times to split each hash signature in the LSH algorithm
    /// (i.e., number of hash tables).
    pub fn new(records: &'a Vec<Record>, num_perm: usize, num_bands: usize) -> Self {
        let mut rng = StdRng::from_entropy();
        let permutations: Vec<(u64, u64)> = (0..num_perm).map(|_| (rng.gen(), rng.gen())).collect();
        let band_size = num_perm / num_bands;
        let mut minhash_map: HashMap<&str, MinHash> = HashMap::with_capacity(records.len());
        let mut hash_tables: Vec<HashMap<u64, Vec<&str>>> = vec![HashMap::new(); num_bands];
        for Record { id, text } in records {
            let items = text.split_whitespace().collect();
            let minhash = MinHash::new(items, &permutations);
            minhash_map.insert(&id, minhash.clone());
            for (i, table) in hash_tables.iter_mut().enumerate() {
                let start = i * band_size;
                let end = start + band_size;
                let band_hash = calculate_band_hash(&minhash.hash_values[start..end]);
                table.entry(band_hash).or_insert_with(Vec::new).push(id);
            }
        }
        MinHashLSH {
            minhash_map,
            band_size,
            hash_tables,
        }
    }

    /// Query the LSH for (potentially) similar items.
    ///
    /// ## Arguments
    ///
    /// * `minhash` - The MinHash instance to query for.
    /// * `threshold` - threshold (inclusive) for jaccard similarity to apply to query result (optional).
    ///
    pub fn query(&self, minhash: &MinHash, threshold: Option<f64>) -> Vec<&'a str> {
        let candidates: HashSet<&'a str> =
            self.hash_tables
                .iter()
                .enumerate()
                .fold(HashSet::new(), |mut doc_set, (i, table)| {
                    let start = i * self.band_size;
                    let end = start + self.band_size;
                    let band_hash = calculate_band_hash(&minhash.hash_values[start..end]);
                    if let Some(docs) = table.get(&band_hash) {
                        doc_set.extend(docs);
                    }
                    doc_set
                });
        if let Some(threshold) = threshold {
            candidates
                .into_par_iter()
                .filter_map(|id| {
                    let candidate_hash = &self.minhash_map[id];
                    if minhash.jaccard_similarity(candidate_hash) >= threshold {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            candidates.into_iter().collect()
        }
    }
}

#[inline]
fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = FxHasher::default();
    t.hash(&mut s);
    s.finish()
}

#[inline]
fn permute_hash(hash: u64, a: u64, b: u64) -> u32 {
    ((a.wrapping_mul(hash).wrapping_add(b)) >> 32) as u32
}

#[inline]
fn calculate_band_hash(band: &[u32]) -> u64 {
    let mut hasher = FxHasher::default();
    for &value in band {
        hasher.write_u32(value);
    }
    hasher.finish()
}
