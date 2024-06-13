use std::path::Path;

pub mod client;
pub mod crypto;
pub mod model;

#[cfg(unix)]
pub const PLATFORM: model::Platform = model::Platform::Unix;
#[cfg(windows)]
pub const PLATFORM: model::Platform = model::Platform::Windows;
pub const PLATFORM_HEADER: &str = "Platform";

pub fn checksum<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
    Ok(sha256::digest(std::fs::read(path)?.as_slice()))
}

// TODO Make a compression work on Windows
#[cfg(unix)]
pub fn compress(data: &[u8]) -> Vec<u8> {
    // let mut encoder = zstd::Encoder::new(Vec::new(), 0).unwrap();
    // encoder.write_all(data).unwrap();
    // encoder.finish().unwrap()

    lzma::compress(data, 6).unwrap()
}

#[cfg(unix)]
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let mut data = data;
    lzma::decompress(&mut data).unwrap()
}

#[cfg(windows)]
pub fn compress(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}

#[cfg(windows)]
pub fn decompress(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}

// fn xor(data: &[u8], key: &[u8]) -> Vec<u8> {
//     data.iter()
//         .enumerate()
//         .fold(Vec::with_capacity(data.len()), |mut acc, (i, &byte)| {
//             acc.push(byte ^ key[i % key.len()]);
//             acc
//         })
// }

// pub fn pack(data: &[u8], key: &[u8]) -> Vec<u8> {
//     xor(data, key)
// }

// pub fn unpack(data: &[u8], key: &[u8]) -> Vec<u8> {
//     xor(data, key)
// }

fn xor<'a>(data: &'a mut [u8], key: &[u8]) -> &'a mut [u8] {
    data.iter_mut()
        .enumerate()
        .for_each(|(i, byte)| *byte = *byte ^ key[i % key.len()]);
    data
}

pub fn pack<'a>(data: &'a mut [u8], key: &[u8]) -> &'a mut [u8] {
    xor(data, key)
}

pub fn unpack<'a>(data: &'a mut [u8], key: &[u8]) -> &'a mut [u8] {
    xor(data, key)
}

pub fn unpack_clone<'a>(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut data = data.to_vec();
    xor(data.as_mut_slice(), key);
    data
}
