use std::{
    fs, io,
    os::windows::process::CommandExt,
    path::Path,
    process::{self, Child, Command, Output},
};

use common::{model, crypto};

use crate::AgentResult;

const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
const DETACHED_PROCESS: u32 = 0x00000008;
const CREATE_NO_WINDOW: u32 = 0x08000000;

const SIGNING_KEY_PATH: &str = ".hermes"; // TODO %LOCALAPPDATA%

pub fn signing_key() -> AgentResult<crypto::SigningKey> {
    let signing_key = if Path::new(SIGNING_KEY_PATH).exists() {
        crypto::get_signing_key_from(fs::read(SIGNING_KEY_PATH)?.as_slice().try_into().unwrap())
    } else {
        // fs::create_dir_all(SIGNING_KEY_DIR_PATH)?;
        let signing_key = crypto::get_signing_key();
        fs::write(SIGNING_KEY_PATH, signing_key.as_bytes())?;
        signing_key
    };
    Ok(signing_key)
}

pub fn execute_cmd(cmd: &str) -> io::Result<Output> {
    Command::new("cmd")
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS | CREATE_NO_WINDOW)
        .arg("/C")
        .arg(cmd)
        .output()
}

pub fn execute_detached(bin: &Path, mission: model::Mission) -> io::Result<Child> {
    Command::new(&bin)
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
