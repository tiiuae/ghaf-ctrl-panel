use std::cell::{Ref, RefMut, RefCell};
use gtk::{self, gio, glib, prelude::*};
use gio::ListStore;
use std::thread::{self, JoinHandle};
use std::sync::{Arc, Mutex, RwLock, mpsc::{self, Sender, Receiver}, atomic::{AtomicBool, Ordering}};
use tokio::time::{sleep, Duration};
use tokio::runtime::Runtime;

use givc_client::{self, AdminClient};
use givc_common::query::{QueryResult, Event, VMStatus, TrustLevel};
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
                admin_client: Arc::new(AdminClient::new(String::from("127.0.0.1"), 9000, None)),
                //String::from("http://[::1]"), 50051,
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
                    let Ok(result) = admin_client.watch().await else {println!("Watch call error"); return};
                    let list = result.initial.clone();
                    let _ = event_tx.send(EventExtended::InitialList(list));

                    while !(stop_signal.load(Ordering::SeqCst)) {
                        if let Ok(event) = result.channel.try_recv() {//await blocks thread until data comes!
                            let _ = event_tx.send(EventExtended::InnerEvent(event));
                        } else {
                            println!("Error received from client lib!");
                        }
                        println!("Waiting for data...");
                        sleep(Duration::new(2,0)).await;
                    }

                    println!("BreakLoop");
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
                                store_inner.append(&VMGObject::new(vm.name, vm.description, vm.status, vm.trust_level));
                            }
                        },
                        EventExtended::InnerEvent(event) =>
                        match event {
                            Event::UnitStatusChanged(result) => {
                                println!("Status: {:?}", result);
                                let store_inner = store.lock().unwrap();
                                for i in 0..store_inner.n_items() {
                                    if let Some(item) = store_inner.item(0) {
                                        let obj = item.downcast_ref::<VMGObject>().unwrap();
                                        if obj.name() == result.name {
                                            obj.update(result);
                                            break;
                                        }
                                    }
                                }
                                //for some reason func is not found
                                /*store_inner.find_with_equal_func(|item| {
                                    let obj = item.downcast_ref::<VMGObject>().unwrap();
                                    if obj.name() == result.name {
                                        obj.update(result);
                                        true // Return true if the item is found and updated
                                    } else {
                                        false // Continue searching otherwise
                                    }
                                });*/
                            },
                            Event::UnitShutdown(result) => {
                                println!("Shutdown info: {:?}", result);
                            },
                            Event::UnitRegistered(result) => {
                                println!("Unit registered {:?}", result);
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
            vec.push(VMGObject::new("VM1".to_string(), String::from("This is the file.pdf"), VMStatus::Running, TrustLevel::NotSecure));
            vec.push(VMGObject::new("VM2".to_string(), String::from("Google Chrome"), VMStatus::Paused, TrustLevel::Secure));
            vec.push(VMGObject::new("VM3".to_string(), String::from("AppFlowy"), VMStatus::Running, TrustLevel::Secure));
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

        pub fn set_connection_config(&self, addr: String, port: u32) {
            //TODO: reconnect with new addr & port
            //admin_client should be wrapped by smth (RwLock?)
            //or provide function to do it
            //let admin_client_obj = self.admin_client.lock().unwrap();
            //*admin_client_obj = AdminClient::new(addr, port.try_into().unwrap(), None);
            //self.reconnect();
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
                //drop(self.admin_client.clone());
                println!("Client thread stopped!");
            }
        }

        pub fn start_vm(&self, name: String) {
            let admin_client = self.admin_client.clone();
            Runtime::new().unwrap().spawn(async move {//or block_on?
                if let Err(error) = admin_client.start(name).await {
                    println!("Start request error {error}");
                }
                else {
                    println!("Start request sent");
                };
            });
        }

        pub fn pause_vm(&self, name: String) {
            let admin_client = self.admin_client.clone();
            Runtime::new().unwrap().spawn(async move {
                if let Err(error) = admin_client.pause(name).await {
                    println!("Pause request error {error}");
                }
                else {
                    println!("Pause request sent");
                };
            });
        }

        pub fn resume_vm(&self, name: String) {
            let admin_client = self.admin_client.clone();
            Runtime::new().unwrap().spawn(async move {
                if let Err(error) = admin_client.resume(name).await {
                    println!("Resume request error {error}");
                }
                else {
                    println!("Resume request sent");
                };
            });
        }

        pub fn shutdown_vm(&self, name: String) {
            let admin_client = self.admin_client.clone();
            Runtime::new().unwrap().spawn(async move {
                if let Err(error) = admin_client.stop(name).await {
                    println!("Stop request error {error}");
                }
                else {
                    println!("Stop request sent");
                };
            });
        }

        pub fn restart_vm(&self, name: String) {
            todo!();
            //no restart in admin_client
            //self.admin_client.restart(name);
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

