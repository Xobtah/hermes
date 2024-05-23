use std::{
    io,
    os::windows::process::CommandExt,
    path::Path,
    process::{self, Child, Command, Output},
};

use common::api;

const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
const DETACHED_PROCESS: u32 = 0x00000008;
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn execute_cmd(cmd: &str) -> io::Result<Output> {
    Command::new("cmd")
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS | CREATE_NO_WINDOW)
        .arg("/C")
        .arg(cmd)
        .output()
}

pub fn execute_detached(bin: &Path, mission: api::Mission) -> io::Result<Child> {
    Command::new(&bin)
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS | CREATE_NO_WINDOW)
        .arg(serde_json::to_string(&api::Mission {
            task: api::Task::Stop,
            ..mission
        })?)
        .stdin(process::Stdio::inherit())
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .spawn()
}
