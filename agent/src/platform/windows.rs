use std::{
    io,
    os::windows::process::CommandExt,
    path::Path,
    process::{self, Child, Command, Output},
};

use common::model;

const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
const DETACHED_PROCESS: u32 = 0x00000008;
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn execute_cmd(cmd: &str) -> io::Result<Output> {
    Command::new("cmd")
        .creation_flags(CREATE_NO_WINDOW)
        .arg("/C")
        .arg(cmd)
        .output()
    // Command::new("powershell")
    //     .creation_flags(CREATE_NO_WINDOW)
    //     .arg("-Command")
    //     .arg(cmd)
    //     .output()
}

pub fn execute_detached(bin: &Path, mission: model::Mission) -> io::Result<Child> {
    Command::new(bin)
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS | CREATE_NO_WINDOW)
        .arg(serde_json::to_string(&model::Mission {
            task: model::Task::Stop,
            ..mission
        })?)
        .stdin(process::Stdio::inherit())
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .spawn()
}
