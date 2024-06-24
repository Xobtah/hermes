use rand::RngCore;
use std::path::Path;

pub mod client;
pub mod crypto;
pub mod model;

#[cfg(unix)]
pub const PLATFORM: model::Platform = model::Platform::Unix;
#[cfg(windows)]
pub const PLATFORM: model::Platform = model::Platform::Windows;
pub const PLATFORM_HEADER: &str = "Platform";

const XOR_KEY_SIZE: usize = 16;

pub fn checksum<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
    Ok(sha256::digest(std::fs::read(path)?.as_slice()))
}

// TODO Try better compressions
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

pub fn pack_to_vec(data: &[u8]) -> Vec<u8> {
    let mut rng = rand::rngs::OsRng {};
    let mut key = [0u8; XOR_KEY_SIZE];
    rng.fill_bytes(&mut key);

    let mut data = compress(data);
    xor(&mut data, &key);
    [[XOR_KEY_SIZE as u8].as_slice(), &key, data.as_slice()].concat()
}

pub fn unpack_to_vec(data: &[u8]) -> Vec<u8> {
    if data.is_empty() || data[1..].len() <= data[0] as usize {
        return vec![];
    }

    let key = &data[1..][..data[0] as usize];
    let mut data = data[data[0] as usize + 1..].to_vec();
    xor(&mut data, &key);
    decompress(&data)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pack_unpack() {
        let data = "Coucou, ça va ?";
        let packed = super::pack_to_vec(data.as_bytes());
        let unpacked = super::unpack_to_vec(&packed);
        assert_eq!(String::from_utf8(unpacked).unwrap(), data);
        let data = "Super, et toi ?";
        let packed = super::pack_to_vec(data.as_bytes());
        let unpacked = super::unpack_to_vec(&packed);
        assert_eq!(String::from_utf8(unpacked).unwrap(), data);
    }
}
