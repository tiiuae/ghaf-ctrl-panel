use std::cell::{Ref, RefMut, RefCell};
use gtk::prelude::*;
use gtk::{self, gio, glib};
use gio::ListStore;
use glib::subclass::prelude::*;

use crate::vm_gobject::VMGObject;

use tonic::{Request, Response, Status};

use admin::admin_service_client::{AdminServiceClient};
use admin::{ApplicationRequest, ApplicationResponse, UnitStatus};

//to communicate with admin service and get data
pub mod admin {
    tonic::include_proto!("admin");
}

pub mod imp {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct DataProvider {
        pub store: RefCell<ListStore>,
        pub status: bool,
    }

    impl DataProvider {
        pub fn new() -> Self {
            let init_store = Self::fill_by_mock_data();

            Self {
                store: RefCell::new(init_store),
                status: false,
            }
        }

        fn fill_by_mock_data() -> ListStore {
            let init_store = ListStore::new::<VMGObject>();
            let mut vec: Vec<VMGObject> = Vec::new();
            vec.push(VMGObject::new("VM1".to_string(), String::from("This is the file.pdf")));
            vec.push(VMGObject::new("VM2".to_string(), String::from("Google Chrome")));
            init_store.extend_from_slice(&vec);
            return init_store;
        }

        pub fn get_store_copy(&self) -> ListStore {
            self.store.borrow().clone()
        }

        pub fn get_store_ref(&self) -> Ref<ListStore> {
            self.store.borrow()
        }

        pub fn get_store_mut_ref(&self) -> RefMut<ListStore> {
            self.store.borrow_mut()
        }

        pub fn add_vm(&self, vm: VMGObject) {
            let mut store = self.store.borrow_mut();
            store.append(&vm);
        }

        pub fn test_request(&self) {
            println!("Test request...");
            //call client::regisrty() !!!
            //call client::app_request() !!!
        }

        //needs separate thread?
        async fn app_request() -> Result<(), Box<dyn std::error::Error>> {
            //beter create and move it to another thread?
            let mut client = AdminServiceClient::connect("http://[::1]:50051").await?;
        
            let request = tonic::Request::new(ApplicationRequest {
                app_name: "Test".into(),
            });
        
            let response = client.application_status(request).await?;
        
            println!("RESPONSE={:?}", response);
            Ok(())
        }        
    }

    impl Default for DataProvider {
        fn default() -> Self {
            Self::new()
        }
    }
}

