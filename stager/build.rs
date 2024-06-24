use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=..");

    let target = env::var("TARGET").unwrap();
    let host = env::var("HOST").unwrap();
    let profile = env::var("PROFILE").unwrap();
    let packer_bin_name = "packer.exe";
    let agent_bin_name = "agent.exe";

    let packer_path = if target == host {
        format!("../target/{profile}/{packer_bin_name}")
    } else {
        format!("../target/{target}/{profile}/{packer_bin_name}")
    };
    let agent_path = if target == host {
        format!("../target/{profile}/{agent_bin_name}")
    } else {
        format!("../target/{target}/{profile}/{agent_bin_name}")
    };

    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR env var not set");
    let out_dir = Path::new(&out_dir);

    if !std::path::Path::new(&packer_path).is_file() {
        panic!("File not found '{packer_path}'");
    }
    if !std::path::Path::new(&agent_path).is_file() {
        panic!("File not found '{agent_path}'");
    }

    fs::write(out_dir.join("packer.exe"), fs::read(packer_path)?)?;
    fs::write(
        out_dir.join("agent.pack"),
        common::pack_to_vec(&fs::read(agent_path)?),
    )?;
    Ok(())
}
