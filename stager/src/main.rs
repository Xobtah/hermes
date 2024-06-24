use std::{env, fs};

use common::crypto;
use object::{pe::ImageNtHeaders64, read::pe::PeFile, Object as _, ObjectSection as _};

mod win_h;

const PACKER_STUB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/packer.exe"));
const AGENT_PACK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/agent.pack"));

fn section_file_range(file: &PeFile<ImageNtHeaders64>, name: &str) -> Option<(u64, u64)> {
    return file.sections().filter(|s| s.name().is_ok()).find_map(|s| {
        if s.name() == Ok(name) {
            s.file_range()
        } else {
            None
        }
    });
}

fn secret_key_mut(agent: &mut [u8]) -> Result<&mut [u8], Box<dyn std::error::Error>> {
    let agent_pe = PeFile::<ImageNtHeaders64>::parse(&agent)?;
    let (offset, size) = section_file_range(&agent_pe, obfstr::obfstr!(".sk")).unwrap();
    Ok(&mut agent[offset as usize..][..size as usize])
}

fn align(size: usize, alignment: usize) -> usize {
    (size as f32 / alignment as f32).ceil() as usize * alignment
}

unsafe fn resize_reloc(mut bin: Vec<u8>) -> Vec<u8> {
    let base = bin.as_mut_ptr();
    let dos_header = base as *mut win_h::IMAGE_DOS_HEADER;
    let nt_header =
        (base as usize + (*dos_header).e_lfanew as usize) as *mut win_h::IMAGE_NT_HEADER;
    let file_alignment = (*nt_header).OptionalHeader.FileAlignment;
    let sections = nt_header.offset(1) as *mut win_h::IMAGE_SECTION_HEADER;
    let reloc_section = sections.offset(((*nt_header).FileHeader.NumberOfSections - 1) as isize);

    // If the .reloc section is too small to receive another entry, make it bigger
    if (*reloc_section).SizeOfRawData - (*reloc_section).Misc.VirtualSize < 0xC {
        (*reloc_section).SizeOfRawData += file_alignment;
        bin.extend(vec![0; file_alignment as usize]);
    }

    bin
}

unsafe fn add_section(mut bin: Vec<u8>, data: &[u8]) -> Vec<u8> {
    let base = bin.as_mut_ptr();
    let dos_header = base as *mut win_h::IMAGE_DOS_HEADER;
    let nt_header =
        (base as usize + (*dos_header).e_lfanew as usize) as *mut win_h::IMAGE_NT_HEADER;
    let section_alignment = (*nt_header).OptionalHeader.SectionAlignment;
    let file_alignment = (*nt_header).OptionalHeader.FileAlignment;
    let sections = nt_header.offset(1) as *mut win_h::IMAGE_SECTION_HEADER;
    let penultimate_section =
        sections.offset(((*nt_header).FileHeader.NumberOfSections - 1) as isize);
    let ultimate_section = sections.offset((*nt_header).FileHeader.NumberOfSections as isize);

    // Sets the new section containing the packed data
    *ultimate_section = win_h::IMAGE_SECTION_HEADER {
        Name: *b".mdr\0\0\0\0",
        Misc: win_h::IMAGE_SECTION_HEADER_0 {
            VirtualSize: data.len() as u32,
        },
        VirtualAddress: align(
            ((*penultimate_section).VirtualAddress + (*penultimate_section).Misc.VirtualSize)
                as usize,
            section_alignment as usize,
        ) as u32,
        SizeOfRawData: align(data.len(), file_alignment as usize) as u32,
        PointerToRawData: align(
            ((*penultimate_section).PointerToRawData + (*penultimate_section).SizeOfRawData)
                as usize,
            file_alignment as usize,
        ) as u32,
        PointerToRelocations: 0,
        PointerToLinenumbers: 0,
        NumberOfRelocations: 0,
        NumberOfLinenumbers: 0,
        Characteristics: 0x40000040,
    };

    // NumberOfSections++
    (*nt_header).FileHeader.NumberOfSections += 1;

    // Adjust size of image
    (*nt_header).OptionalHeader.SizeOfImage = align(
        ((*ultimate_section).VirtualAddress + (*ultimate_section).Misc.VirtualSize) as usize,
        section_alignment as usize,
    ) as u32;

    // Append the packed data to the packer
    bin.extend(data);
    bin.extend(vec![
        0;
        (*ultimate_section).SizeOfRawData as usize - data.len()
    ]);

    bin
}

unsafe fn set_reference_to_data(mut bin: Vec<u8>, data: &[u8]) -> Vec<u8> {
    let base = bin.as_mut_ptr();
    let dos_header = base as *mut win_h::IMAGE_DOS_HEADER;
    let nt_header =
        (base as usize + (*dos_header).e_lfanew as usize) as *mut win_h::IMAGE_NT_HEADER;
    let sections = nt_header.offset(1) as *mut win_h::IMAGE_SECTION_HEADER;
    let ultimate_section = sections.offset(((*nt_header).FileHeader.NumberOfSections - 1) as isize);

    // Set reference to new section
    let pe = PeFile::<ImageNtHeaders64>::parse(&bin).unwrap();
    let (offset, size) = section_file_range(&pe, ".bin").unwrap();
    bin[offset as usize..][..size as usize].copy_from_slice(
        &[
            ((*nt_header).OptionalHeader.ImageBase + (*ultimate_section).VirtualAddress as u64)
                .to_le_bytes(),
            data.len().to_le_bytes(),
        ]
        .concat(),
    );

    bin
}

unsafe fn modify_reloc(mut bin: Vec<u8>) -> Vec<u8> {
    let base = bin.as_mut_ptr();
    let dos_header = base as *mut win_h::IMAGE_DOS_HEADER;
    let nt_header =
        (base as usize + (*dos_header).e_lfanew as usize) as *mut win_h::IMAGE_NT_HEADER;
    let sections = nt_header.offset(1) as *mut win_h::IMAGE_SECTION_HEADER;
    let reloc_section_header =
        sections.offset(((*nt_header).FileHeader.NumberOfSections - 2) as isize);
    let bin_section_header = sections.offset(2 as isize);
    let reloc_section = (bin.as_mut_ptr() as *mut u8)
        .offset((*reloc_section_header).PointerToRawData as isize)
        as *mut win_h::IMAGE_BASE_RELOCATION;

    // Build the list of the relocations
    let mut relocs = vec![];
    let mut relocation = reloc_section;
    while (*relocation).SizeOfBlock != 0 {
        relocs.push(
            std::slice::from_raw_parts(relocation as *mut u8, (*relocation).SizeOfBlock as usize)
                .to_vec(),
        );
        relocation = (relocation as *const u8).add((*relocation).SizeOfBlock as usize)
            as *mut win_h::IMAGE_BASE_RELOCATION;
    }

    // Add the reloc block for the .bin section
    relocs.push(
        vec![
            (*bin_section_header)
                .VirtualAddress
                .to_le_bytes()
                .as_slice(),
            (0xc as u32).to_le_bytes().as_slice(),
            (0xa000 as u16).to_le_bytes().as_slice(),
            (0x00 as u16).to_le_bytes().as_slice(),
        ]
        .concat(),
    );

    // Sort the relocs by VirtualAddress INC
    relocs.sort_by(|a, b| {
        let a = a.as_ptr() as *const win_h::IMAGE_BASE_RELOCATION;
        let b = b.as_ptr() as *const win_h::IMAGE_BASE_RELOCATION;
        (*a).VirtualAddress
            .partial_cmp(&(*b).VirtualAddress)
            .unwrap()
    });

    // Copy the newly computed relocations to the .reloc section header
    let relocations_header = relocs.concat();
    std::slice::from_raw_parts_mut(reloc_section as *mut u8, relocations_header.len())
        .copy_from_slice(&relocations_header);

    // Adjust .reloc section header sizes
    (*nt_header).OptionalHeader.DataDirectory[5].Size = relocations_header.len() as u32;
    (*reloc_section_header).Misc.VirtualSize = relocations_header.len() as u32;

    bin
}

unsafe fn calculate_checksum(bin: &mut [u8]) {
    let base = bin.as_mut_ptr();
    let dos_header = base as *mut win_h::IMAGE_DOS_HEADER;
    let nt_header =
        (base as usize + (*dos_header).e_lfanew as usize) as *mut win_h::IMAGE_NT_HEADER;

    let checksum_offset = (*dos_header).e_lfanew as usize
        + std::mem::size_of::<win_h::IMAGE_FILE_HEADER>()
        + std::mem::size_of::<u32>()
        + 64usize;
    let eof = bin.len();
    let mut checksum = 0u64;

    for offset in (0..eof).step_by(4) {
        if offset == checksum_offset {
            continue;
        }
        let data = *(bin.as_ptr() as *const u32).offset(offset as isize);
        checksum = (checksum & 0xFFFFFFFF) + (data as u64) + (checksum >> 32);
        if checksum > (u32::MAX as u64) {
            checksum = (checksum & 0xFFFFFFFF) + (checksum >> 32);
        }
    }

    checksum = (checksum & 0xFFFF) + (checksum >> 16);
    checksum = checksum + (checksum >> 16);
    checksum = checksum & 0xFFFF;
    checksum += eof as u64;

    (*nt_header).OptionalHeader.CheckSum = checksum as u32;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set a secret key in the agent
    let mut agent = common::unpack_to_vec(&AGENT_PACK);
    secret_key_mut(&mut agent)?.copy_from_slice(crypto::get_signing_key().as_bytes());
    let packed_agent = common::pack_to_vec(&agent);

    // http://www.sunshine2k.de/reversing/tuts/tut_addsec.htm
    let packer = unsafe {
        let packer = resize_reloc(PACKER_STUB.to_vec());
        let packer = add_section(packer, &packed_agent);
        let packer = set_reference_to_data(packer, &packed_agent);
        let mut packer = modify_reloc(packer);
        calculate_checksum(&mut packer);
        packer
    };

    // Replace the current executable with the updated one
    let tmp = env::current_exe()?.with_extension(obfstr::obfstr!("tmp"));
    fs::write(&tmp, packer)?;
    self_replace::self_replace(&tmp)?;
    fs::remove_file(&tmp)?;

    Ok(())
}
