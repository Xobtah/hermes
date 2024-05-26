use std::{
    io,
    os::unix::process::CommandExt,
    path::Path,
    process::{self, Child, Command, Output},
};

use common::model;

pub fn execute_cmd(cmd: &str) -> io::Result<Output> {
    Command::new("sh").arg("-c").arg(cmd).output()
}

pub fn execute_detached(bin: &Path, mission: model::Mission) -> io::Result<Child> {
    unsafe {
        Command::new(&bin)
            .arg(serde_json::to_string(&model::Mission {
                task: model::Task::Stop,
                ..mission
            })?)
            .pre_exec(|| {
                libc::setsid();
                Ok(())
            })
            .stdin(process::Stdio::inherit())
            .stdout(process::Stdio::inherit())
            .stderr(process::Stdio::inherit())
            .spawn()
    }
}
