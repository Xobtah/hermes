use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=.");

    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_resource::compile("dropper-manifest.rc", embed_resource::NONE);
    }

    let target = env::var("TARGET").unwrap();
    let host = env::var("HOST").unwrap();
    let profile = env::var("PROFILE").unwrap();
    let bin_name = "agentp.exe";

    let packer_path = if target == host {
        format!("../target/{profile}/{bin_name}")
    } else {
        format!("../target/{target}/{profile}/{bin_name}")
    };

    if !std::path::Path::new(&packer_path).is_file() {
        panic!("File not found '{packer_path}'");
    }

    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR env var not set");
    let out_dir = Path::new(&out_dir);

    fs::write(out_dir.join("agentp"), fs::read(packer_path)?)?;
    Ok(())
}
