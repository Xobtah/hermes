use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=.");
    let xor_key = "ABCDEFGHIKLMNOPQRSTVXYZ"; // TODO Generate random key

    let target = env::var("TARGET").unwrap();
    let host = env::var("HOST").unwrap();
    let profile = env::var("PROFILE").unwrap();
    let bin_name = "agent.exe";

    let agent_path = if target == host {
        format!("../target/{profile}/{bin_name}")
    } else {
        format!("../target/{target}/{profile}/{bin_name}")
    };

    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR env var not set");
    let out_dir = Path::new(&out_dir);

    if !std::path::Path::new(&agent_path).is_file() {
        panic!("File not found '{agent_path}'");
    }

    let mut agent_bin = fs::read(agent_path)?;
    common::pack(&mut agent_bin, xor_key.as_bytes());

    fs::write(out_dir.join("xor_key"), xor_key.as_bytes())?;
    fs::write(out_dir.join("agent_xor"), agent_bin)?;
    Ok(())
}
