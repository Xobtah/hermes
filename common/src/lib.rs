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

pub fn compress(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec(data, 6)
}

pub fn decompress(data: &[u8]) -> Vec<u8> {
    miniz_oxide::inflate::decompress_to_vec(data).expect("Failed to decompress")
}

fn xor<'a>(data: &'a mut [u8], key: &[u8]) -> &'a mut [u8] {
    data.iter_mut()
        .enumerate()
        .for_each(|(i, byte)| *byte ^= key[i % key.len()]);
    data
}

pub fn pack_to_vec(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut data = compress(data);
    xor(&mut data, key);
    data
}

pub fn unpack_to_vec(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut data = data.to_vec();
    xor(&mut data, key);
    decompress(&data)
}
