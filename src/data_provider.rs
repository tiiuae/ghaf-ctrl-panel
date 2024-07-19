use std::cell::{Ref, RefMut, RefCell};
use gtk::{self, gio, glib};
use gio::ListStore;
use std::thread::{self, JoinHandle};
use std::sync::{Arc, Mutex, mpsc::{self, Sender, Receiver}, atomic::{AtomicBool, Ordering}};
//use tokio::time::{sleep, Duration};
use tokio::runtime::Runtime;

use givc_client::{self, AdminClient, client::QueryResult, client::Event};
//use givc_client::endpoint::{EndpointConfig, TlsConfig};
//use givc_common::types::*;

use crate::vm_gobject::VMGObject;
use crate::settings_gobject::SettingsGObject;

pub mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct DataProvider {
        pub store: Arc<Mutex<ListStore>>,
        pub settings: Arc<Mutex<SettingsGObject>>,
        pub status: bool,
        pub admin_client: Arc<AdminClient>,
        pub handle: RefCell<Option<JoinHandle<()>>>,
        stop_signal: Arc<AtomicBool>,
    }

    #[derive(Debug)]
    pub enum EventExtended {
        InitialList(Vec<QueryResult>),
        InnerEvent(Event),
        BreakLoop,
    }

    impl DataProvider {
        pub fn new() -> Self {
            let init_store = Self::fill_by_mock_data();//ListStore::new::<VMGObject>();

            Self {
                store: Arc::new(Mutex::new(init_store)),
                settings: Arc::new(Mutex::new(SettingsGObject::default())),
                status: false,
                admin_client: Arc::new(AdminClient::new(String::from("http://[::1]"), 50051, None)),
                handle: RefCell::new(None),
                stop_signal: Arc::new(AtomicBool::new(false)),
            }
        }

        pub fn establish_connection(&self) {
            let admin_client = self.admin_client.clone();
            let store = self.store.clone();
            let stop_signal = self.stop_signal.clone();
            self.stop_signal.store(false, Ordering::SeqCst);

            let (event_tx, event_rx): (Sender<EventExtended>, Receiver<EventExtended>) = mpsc::channel();

            let handle = thread::spawn(move || {
                Runtime::new().unwrap().block_on(async move {
                    let Ok(result) = admin_client.watch().await else {todo!()};
                    let list = result.initial;
                    let _ = event_tx.send(EventExtended::InitialList(list));

                    let channel = result.channel;

                    while !stop_signal.load(Ordering::SeqCst) {
                        if let Ok(event) = channel.recv().await {
                            let _ = event_tx.send(EventExtended::InnerEvent(event));
                        } else {
                            println!("Error received from client lib!");
                            break;
                        }
                    }

                    let _ = event_tx.send(EventExtended::BreakLoop);
                });
            });

            *self.handle.borrow_mut() = Some(handle);

            glib::source::idle_add_local(move || {
                while let Ok(event_ext) = event_rx.try_recv() {
                    match event_ext {
                        EventExtended::InitialList(list) => {
                            println!("Initial list: {:?}", list);
                            let store_inner = store.lock().unwrap();
                            store_inner.remove_all();
                            for vm in list {
                                store_inner.append(&VMGObject::new(vm.name, vm.description, 0, 0));
                                //TODO: add convert functions for other fields!
                            }
                        },
                        EventExtended::InnerEvent(event) =>
                        match event {
                            Event::UnitStatusChanged(status) => {
                                println!("Status: {:?}", status);
                            },
                            Event::UnitShutdown(info) => {
                                println!("Shutdown info: {}", info);
                            },
                        },
                        EventExtended::BreakLoop => {
                            println!("BreakLoop event received");
                            break;
                        }
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        fn fill_by_mock_data() -> ListStore {
            let init_store = ListStore::new::<VMGObject>();
            let mut vec: Vec<VMGObject> = Vec::new();
            vec.push(VMGObject::new("VM1".to_string(), String::from("This is the file.pdf"), 0, 2));
            vec.push(VMGObject::new("VM2".to_string(), String::from("Google Chrome"), 0, 0));
            init_store.extend_from_slice(&vec);
            return init_store;
        }

        pub fn get_store(&self) -> ListStore {
            self.store.lock().unwrap().clone()
        }

        pub fn get_store_ref(&self) -> Arc<Mutex<ListStore>> {
            self.store.clone()
        }

        pub fn add_vm(&self, vm: VMGObject) {
            let store = self.store.lock().unwrap();
            store.append(&vm);
        }

        pub fn reconnect(&self) {
            println!("Reconnect request...");

            self.disconnect();
            self.establish_connection();
        }

        pub fn disconnect(&self) {
            println!("Disconnect request...");
            self.stop_signal.store(true, Ordering::SeqCst);
            if let Some(handle) = self.handle.borrow_mut().take() {
                handle.join().unwrap();
            }
            println!("Client thread stopped!");
        }
    }

    impl Default for DataProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Drop for DataProvider {
        fn drop(&mut self) {
            println!("DataProvider is about to drop");
            self.disconnect();
        }
    }
}

