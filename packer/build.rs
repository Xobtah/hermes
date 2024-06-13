use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=.");
    let xor_key = "ABCDEFGHIKLMNOPQRSTVXYZ"; // TODO Generate random key

    #[cfg(debug_assertions)]
    let agent_path = "../target/x86_64-pc-windows-gnu/debug/agent.exe";
    #[cfg(not(debug_assertions))]
    let agent_path = "../target/x86_64-pc-windows-gnu/release/agent.exe";

    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR env var not set");
    let out_dir = Path::new(&out_dir);

    let mut agent_bin = fs::read(agent_path)?;
    common::pack(&mut agent_bin, xor_key.as_bytes());

    fs::write(out_dir.join("xor_key"), xor_key.as_bytes())?;
    fs::write(out_dir.join("agent_xor"), agent_bin)?;
    Ok(())
}
