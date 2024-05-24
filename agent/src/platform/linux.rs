use std::{
    fs, io,
    os::unix::process::CommandExt,
    path::Path,
    process::{self, Child, Command, Output},
};

use common::{model, crypto};

use crate::AgentResult;

const SIGNING_KEY_PATH: &str = "/Users/sylvain/.hermes"; // TODO Change it

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
