use rayon::iter::ParallelBridge;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, trace};
use xor_utils::avg_normalized_hamming_distance;
pub const EMK_MAGIC: u64 = 0xAFF24C9CE9EA9943;

#[tracing::instrument(skip(data))]
pub fn xor(data: &[u8], key: &[u8]) -> Result<Vec<u8>, &'static str> {
    trace!("XORing data with key: {:X?}", key);
    let result = data
        .iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect::<Vec<u8>>();

    let magic = b".SFDS";
    if result.starts_with(magic) {
        Ok(result)
    } else {
        Err("Invalid magic")
    }
}

// does the same thing as xor, but only takes the first 5 bytes of the data and verify
// #[tracing::instrument(skip(data))]
pub fn xor_verify(data: &[u8], key: &[u8]) -> bool {
    // trace!("XORing data with key: {:X?}", key);
    let result = data
        .iter()
        .take(5)
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect::<Vec<u8>>();

    let magic = b".SFDS";
    result.starts_with(magic)
}

// /// Attempts to brute-force the XOR key for the given data, iterating every single u64 value
// ///
// /// WARNING: Very slow... Try to avoid using this function
// pub fn xor_cracker(data: Vec<u8>) -> Result<u64, &'static str> {
//     use rayon::prelude::*;
//     // todo: do something like wiremask's xor_cracker

//     let data = data.into_iter().take(5).collect::<Vec<u8>>();

//     let result = (0..=u64::MAX).into_par_iter().find_any(|&key| {
//         println!("Trying key: {:X}", key);
//         let key_bytes = key.to_be_bytes();
//         let key = key_bytes.iter().take(5).cloned().collect::<Vec<u8>>();
//         xor_verify(&data, &key)
//     });

//     result.ok_or("No valid key found")
// }

/// Attempts to brute-force the XOR key for the given data, iterating every single u64 value
///
/// WARNING: Very slow... Try to avoid using this function
pub fn xor_cracker_bruteforce(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    use rayon::prelude::*;

    let max_threads = rayon::max_num_threads();

    info!("Max threads: {}", max_threads);

    let d = avg_normalized_hamming_distance(&data.to_vec(), 16);
    info!("{d:#?}");

    // sort by value (ascending)
    let mut d = d.into_iter().collect::<Vec<_>>();
    d.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // get closest distance
    let (keysize, _) = d.first().unwrap();

    info!("Keysize: {}", keysize);

    info!("Cracking... by {} bytes", keysize);
    // generate all possible keys by keysize

    // generate a random key of keysize, then try with xor_verify

    let total_attempts = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();

    let keys = (0..=u64::MAX).into_par_iter().filter_map(|key| {
        let key_bytes = key.to_be_bytes();
        let key = key_bytes
            .iter()
            .take(*keysize)
            .cloned()
            .collect::<Vec<u8>>();
        total_attempts.fetch_add(1, Ordering::Relaxed);
        Some(key)
    });

    let log_attempts = Arc::clone(&total_attempts);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(5));
        let attempts = log_attempts.load(Ordering::Relaxed);
        let elapsed = start_time.elapsed().as_secs_f64();
        let hashrate = (attempts as f64) / elapsed / 1_000_000.0;
        info!("Attempts: {}, Hashrate: {:.2} MH/s", attempts, hashrate);
    });

    let result = keys.find_any(|key| {
        // trace!("Trying key: {:X?}", key);
        if xor_verify(data, key) {
            info!("Found key: {:X?}", key);
            true
        } else {
            false
        }
    });

    if let Some(key) = result {
        Ok(key)
    } else {
        Err("No valid key found")
    }
}

/// A modified XOR cracker that assumes the file contains mostly zeros,
/// using Alula's algorithm for most bytes, but brute-forces the 4th and 6th bytes.
///
/// Credits @alula on GitHub <3
///
// todo: Probably needs a more reliable way...
pub fn xor_cracker_alula(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    let d = avg_normalized_hamming_distance(&data.to_vec(), 16);
    info!("{d:#?}");

    // sort by value (ascending)
    let mut d = d.into_iter().collect::<Vec<_>>();
    d.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // get closest distance
    let (key_length, _) = d.first().unwrap();
    // let key_length = 8; // Based on the known key length
    let mut key = vec![0u8; *key_length];
    let mut most_common_bytes = vec![0u8; *key_length];

    // Compute most common bytes for each position
    most_common_bytes
        .iter_mut()
        .enumerate()
        .for_each(|(i, byte)| {
            // Get every nth byte (where n is the key position)
            let bytes: Vec<u8> = data.iter().skip(i).step_by(*key_length).cloned().collect();

            // Most common byte is likely to be XOR'd with zero
            *byte = bytes
                .iter()
                .fold(HashMap::new(), |mut map, &b| {
                    *map.entry(b).or_insert(0) += 1;
                    map
                })
                .into_iter()
                .max_by_key(|&(_, count)| count)
                .map(|(b, _)| b)
                .unwrap_or(0);
        });

    // Copy the most common bytes to key
    key.copy_from_slice(&most_common_bytes);

    // The positions to brute-force (0-based indexing)
    let brute_force_positions = [3, 5]; // 4th and 6th bytes

    // Iterate over all possible combinations of the 4th and 6th bytes
    for b4 in 0u8..=255 {
        for b6 in 0u8..=255 {
            key[brute_force_positions[0]] = b4;
            key[brute_force_positions[1]] = b6;

            // Verify the key
            if !xor_verify(data, &key) {
                continue;
            }

            debug!("Found possible key: {:X?}", key);
            // Also verify with from_bytes_with_key
            if crate::types::EmkFile::from_bytes_with_key(data, &key).is_ok() {
                info!("Found key! {:X?}", key);

                return Ok(key);
            }
        }
    }

    Err("No valid key found")
}

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    static VALID_KEY: [u8; 8] = [0xAF, 0xF2, 0x4C, 0x9C, 0xE9, 0xEA, 0x99, 0x43];

    #[traced_test]
    #[test]
    fn test_xor() {
        let data = include_bytes!("../examples/000001.emk");
        let key = VALID_KEY;
        let result = super::xor(data, &key).unwrap();
        assert_eq!(result.len(), data.len());
    }

    #[traced_test]
    #[test]
    fn test_alula_xor_cracker() {
        let data = include_bytes!("../examples/000001.emk");
        let key = super::xor_cracker_alula(data).unwrap();
        assert_eq!(key, VALID_KEY);
    }
}
