use log::{debug, error, info};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct WireGuardVMs {
    list: Vec<String>,
}

impl WireGuardVMs {
    fn new(file_path: PathBuf) -> Self {
        match fs::read_to_string(&file_path) {
            Ok(content) => {
                let list = content.lines().map(|line| line.to_string()).collect();
                info!("Wireguard VMs file has been read, list: {:?}", list);
                WireGuardVMs { list }
            }
            Err(e) => {
                error!("Failed to read the file '{}': {}", file_path.display(), e);
                WireGuardVMs { list: Vec::new() }
            }
        }
    }

    fn get_list(&self) -> &Vec<String> {
        &self.list
    }

    fn contains(&self, query: &String) -> bool {
        self.list.contains(&query)
    }
}

// Static instance initialized once
pub static WVM_LIST: OnceLock<WireGuardVMs> = OnceLock::new();

pub fn initialize_wvm_list(file_path: PathBuf) {
    WVM_LIST.get_or_init(|| WireGuardVMs::new(file_path));
}

pub fn get_static_list() -> &'static Vec<String> {
    &WVM_LIST.get().expect("WVM_LIST is not initialized").list
}

pub fn static_contains(query: &String) -> bool {
    let is_contains = WVM_LIST
        .get()
        .expect("WVM_STRINGS is not initialized")
        .contains(&query);

    debug!("Checking if {} is in the list:{}", query, is_contains);
    is_contains
}
