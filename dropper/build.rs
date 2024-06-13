use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    embed_resource::compile("dropper-manifest.rc", embed_resource::NONE);
    println!("cargo::rerun-if-changed=.");

    #[cfg(debug_assertions)]
    let packer_path = "../target/x86_64-pc-windows-gnu/debug/agentp.exe";
    #[cfg(not(debug_assertions))]
    let packer_path = "../target/x86_64-pc-windows-gnu/release/agentp.exe";

    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR env var not set");
    let out_dir = Path::new(&out_dir);

    fs::write(out_dir.join("agentp"), fs::read(packer_path)?)?;
    Ok(())
}
