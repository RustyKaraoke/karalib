use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, trace};
use xor_utils::avg_normalized_hamming_distance;
const MAGIC_BYTES: &[u8; 5] = b".SFDS";

pub const EMK_MAGIC: u64 = 0xAFF24C9CE9EA9943;

#[tracing::instrument(skip(data))]
pub fn xor(data: &[u8], key: &[u8]) -> Result<Vec<u8>, &'static str> {
    trace!("XORing data with key: {:X?}", key);

    if data.len() < MAGIC_BYTES.len() {
        return Err("Data too short");
    }

    // Verify magic bytes first
    for i in 0..MAGIC_BYTES.len() {
        if (data[i] ^ key[i % key.len()]) != MAGIC_BYTES[i] {
            return Err("Invalid magic");
        }
    }

    // Process full data only if magic bytes match
    Ok(data
        .iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect())
}

pub fn xor_verify(data: &[u8], key: &[u8]) -> bool {
    if data.len() < MAGIC_BYTES.len() {
        return false;
    }

    // Check magic bytes
    let magic_matches = (0..MAGIC_BYTES.len()).all(|i| {
        (data[i] ^ key[i % key.len()]) == MAGIC_BYTES[i]
    });

    if !magic_matches {
        return false;
    }

    // Verify full data structure
    crate::types::EmkReader::decrypt(data, key).is_ok()
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
/// While this is faster than brute-forcing the entire key, it's not as reliable and may fail. Untested with newer files
/// that come with different keys.
///
/// Credits @alula on GitHub <3
///
/// todo: Probably needs a more reliable way...
pub fn xor_cracker_alula(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    // Get optimal key length using hamming distance
    let d = avg_normalized_hamming_distance(&data.to_vec(), 16);
    let (key_length, _) = d
        .into_iter()
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();

    // Pre-allocate key vector
    let mut key = vec![0u8; key_length];

    // Process bytes in parallel using rayon
    key.par_iter_mut().enumerate().for_each(|(i, byte)| {
        // Get frequency of bytes at this position
        let freq = data.iter().skip(i).step_by(key_length).fold(
            HashMap::with_capacity(256),
            |mut map, &b| {
                *map.entry(b).or_insert(0) += 1;
                map
            },
        );

        // Most common byte is likely XORed with zero
        *byte = freq
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(b, _)| b)
            .unwrap_or(0);
    });

    // Try different position combinations in parallel
    let positions: Vec<(usize, usize)> = (0..key.len())
        .flat_map(|i| (i + 1..key.len()).map(move |j| (i, j)))
        .collect();

    let result = positions.into_par_iter().find_map_any(|(pos1, pos2)| {
        (0..=u16::MAX).into_par_iter().find_map_any(|n| {
            let mut test_key = key.clone();
            test_key[pos1] = (n & 0xFF) as u8;
            test_key[pos2] = (n >> 8) as u8;

            if xor_verify(data, &test_key) {
                info!(
                    "Found key with positions [{}, {}]: {:X?}",
                    pos1, pos2, test_key
                );
                Some(test_key)
            } else {
                None
            }
        })
    });

    result.ok_or("No valid key found")
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
