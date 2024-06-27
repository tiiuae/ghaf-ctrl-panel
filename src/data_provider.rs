use std::cell::{Ref, RefMut, RefCell};
use gtk::prelude::*;
use gtk::{self, gio, glib};
use glib::subclass::prelude::*;

use crate::vm_gobject::VMGObject;

pub mod admin {
    tonic::include_proto!("admin");
}

pub mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct DataProvider {
        pub store: RefCell<gtk::gio::ListStore>,
        pub status: bool,
    }

    impl DataProvider {
        pub fn new() -> Self {
            let init_store = gtk::gio::ListStore::new::<VMGObject>();
            let mut vec: Vec<VMGObject> = Vec::new();
            vec.push(VMGObject::new("VM1".to_string(), String::from("This is the file.pdf")));
            vec.push(VMGObject::new("VM2".to_string(), String::from("Google Chrome")));
            init_store.extend_from_slice(&vec);
            Self {
                store: RefCell::new(init_store),
                status: false,
            }
        }

        pub fn get_store_copy(&self) -> gio::ListStore {
            self.store.borrow().clone()
        }

        pub fn get_store_ref(&self) -> Ref<gio::ListStore> {
            self.store.borrow()
        }

        pub fn get_store_mut_ref(&self) -> RefMut<gio::ListStore> {
            self.store.borrow_mut()
        }

        pub fn add_vm(&self, vm: VMGObject) {
            let mut store = self.store.borrow_mut();
            store.append(&vm);
        }
    }

    impl Default for DataProvider {
        fn default() -> Self {
            Self::new()
        }
    }
}

