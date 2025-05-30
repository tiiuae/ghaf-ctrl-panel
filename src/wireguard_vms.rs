use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use crate::prelude::*;

pub struct WireGuardVMs {
    list: Vec<String>,
}

impl WireGuardVMs {
    fn new(file_path: &Path) -> Self {
        match fs::read_to_string(file_path) {
            Ok(content) => {
                let list = content.lines().map(ToString::to_string).collect();
                info!("Wireguard VMs file has been read, list: {list:?}");
                WireGuardVMs { list }
            }
            Err(e) => {
                error!(
                    "Failed to read the file '{path}': {e}",
                    path = file_path.display()
                );
                WireGuardVMs { list: Vec::new() }
            }
        }
    }

    fn contains(&self, query: &str) -> bool {
        self.list.iter().any(|e| e == query)
    }
}

// Static instance initialized once
pub static WVM_LIST: OnceLock<WireGuardVMs> = OnceLock::new();

pub fn initialize_wvm_list(file_path: &Path) {
    WVM_LIST.get_or_init(|| WireGuardVMs::new(file_path));
}

pub fn static_contains(query: &str) -> bool {
    WVM_LIST
        .get()
        .expect("WVM_STRINGS is not initialized")
        .contains(query)
}
