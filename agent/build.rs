use std::env;
use std::fs;
use std::path::Path;

use common::crypto;

fn main() {
    let signing_key = crypto::get_signing_key();
    let target_dir = env::var_os("OUT_DIR").unwrap();
    fs::write(
        Path::new(&target_dir).join("id.key"),
        signing_key.as_bytes(),
    )
    .unwrap();
    fs::write(
        Path::new(&target_dir).join("../../../id-pub.key"),
        signing_key.verifying_key().as_bytes(),
    )
    .unwrap();
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=.");
}
