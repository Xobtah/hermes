#![windows_subsystem = "windows"]
use std::{os::windows::process::CommandExt as _, process::Command};

const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
const DETACHED_PROCESS: u32 = 0x00000008;
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn main() {
    let _ = Command::new("sc.exe")
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS | CREATE_NO_WINDOW)
        .arg("delete")
        .arg("'Agent'")
        .status();
    let _ = Command::new("powershell")
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS | CREATE_NO_WINDOW)
        .arg("-Command")
        .arg("Remove-Item")
        .arg("-Path")
        .arg("'C:\\Windows\\System32\\agent.exe'")
        .arg("-Force")
        .status();
}
