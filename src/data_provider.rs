use std::cell::{Ref, RefMut, RefCell};
use gtk::prelude::*;
use gtk::{self, gio, glib};
use gio::ListStore;
use std::thread;
use std::sync::{Arc, Mutex, mpsc::{self, Sender, Receiver}};
use glib::clone;

use crate::vm_gobject::VMGObject;
use crate::client::client::*;
use crate::client::admin::ApplicationRequest;

pub mod imp {
    use super::*;

    //Idea: put all vm data to separate struct 
    //wrap this struct: Arc<Mutex<Data>>
    //put atomic ref to client's thread
    #[derive(Debug)]
    pub struct DataProvider {
        pub store: Arc<Mutex<ListStore>>,
        pub status: bool,
        pub request_sender: Sender<ClientServiceRequest>,
        pub response_receiver: Arc<Receiver<ClientServiceResponse>>,
    }

    impl DataProvider {
        pub fn new() -> Self {
            let init_store = ListStore::new::<VMGObject>();//Self::fill_by_mock_data();
            let (request_tx, response_rx): (Sender<ClientServiceRequest>, Receiver<ClientServiceResponse>) = Self::make_client_thread();

            Self {
                store: Arc::new(Mutex::new(init_store)),
                status: false,
                request_sender: request_tx,
                response_receiver: Arc::new(response_rx),
            }
        }

        pub fn establish_connection(&self) {

        }

        fn fill_by_mock_data() -> ListStore {
            let init_store = ListStore::new::<VMGObject>();
            let mut vec: Vec<VMGObject> = Vec::new();
            vec.push(VMGObject::new("VM1".to_string(), String::from("This is the file.pdf")));
            vec.push(VMGObject::new("VM2".to_string(), String::from("Google Chrome")));
            init_store.extend_from_slice(&vec);
            return init_store;
        }

        fn make_client_thread() -> (Sender<ClientServiceRequest>, Receiver<ClientServiceResponse>) {
            let (request_tx, request_rx): (Sender<ClientServiceRequest>, Receiver<ClientServiceRequest>) = mpsc::channel();
            let (response_tx, response_rx): (Sender<ClientServiceResponse>, Receiver<ClientServiceResponse>) = mpsc::channel();
            let endpoint = String::from("http://[::1]:50051");

            thread::spawn(move || {
                client_service_thread(endpoint, request_rx, response_tx, Self::response_callback);
            });

            (request_tx, response_rx)
        }

        pub fn get_store(&self) -> ListStore {
            self.store.lock().unwrap().clone()
        }

        pub fn get_store_ref(&self) -> Arc<Mutex<ListStore>> {
            self.store.clone()
        }

        pub fn add_vm(&self, vm: VMGObject) {
            let mut store = self.store.lock().unwrap();
            store.append(&vm);
        }

        pub fn update_request(&self) {
            println!("Update request...");
            //send request
            self.request_sender.send(ClientServiceRequest::AppList()).expect("Send error");
            
            //get response
            let response_receiver = Arc::clone(&self.response_receiver);
            let mut store = Arc::clone(&self.store);
            //tokio::runtime::Runtime::new().unwrap().spawn ?
            glib::spawn_future_local(async move {
                while let Ok(response) = response_receiver.recv() {
                    match response {
                        ClientServiceResponse::AppList(app_list) => {
                            println!("Arc! List: {:?}", app_list.applications);
                            let mut store_inner = store.lock().unwrap();
                            store_inner.remove_all();
                            let mut vec: Vec<VMGObject> = Vec::new();
                            for app in app_list.applications {
                                store_inner.append(&VMGObject::new(app.name, app.description));
                            }
                            break;
                        },
                        _ => todo!(),
                    }
                }
            });
        }
        
        pub fn response_callback(response: ClientServiceResponse) {
            match response {
                ClientServiceResponse::AppList(app_list) => {
                    println!("Callback! List: {:?}", app_list.applications);
                    
                },
                _ => todo!(),
            }
        }
    }

    impl Default for DataProvider {
        fn default() -> Self {
            Self::new()
        }
    }
}

