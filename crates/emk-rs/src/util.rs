use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tracing::{info, trace};
use xor_utils::avg_normalized_hamming_distance;
pub const EMK_MAGIC: u64 = 0xAFF24C9CE9EA9943;

#[tracing::instrument(skip(data))]
pub fn xor(data: &[u8], key: &[u8]) -> Result<Vec<u8>, &'static str> {
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
pub fn xor_verify(data: &[u8], key: &[u8]) -> bool {
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
pub fn xor_cracker(data: &[u8]) -> Result<Vec<u8>, &'static str> {
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

    let keys = (0..=u64::MAX).into_par_iter().filter_map(|key| {
        let key_bytes = key.to_be_bytes();
        let key = key_bytes
            .iter()
            .take(*keysize)
            .cloned()
            .collect::<Vec<u8>>();
        Some(key)
    });

    let result = keys.find_any(|key| {
        trace!("Trying key: {:X?}", key);
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
