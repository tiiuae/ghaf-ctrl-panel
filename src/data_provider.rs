use std::cell::RefCell;
use gtk::{self, gio, glib, prelude::*};
use gio::ListStore;
use std::thread::{self, JoinHandle};
use std::sync::{Arc, Mutex, RwLock, mpsc::{self, Sender, Receiver}, atomic::{AtomicBool, Ordering}};
use tokio::time::{sleep, Duration, timeout};
use tokio::runtime::Runtime;

use givc_client::{self, AdminClient};
use givc_common::query::{QueryResult, Event, VMStatus, TrustLevel};
//use givc_client::endpoint::{EndpointConfig, TlsConfig};
//use givc_common::types::*;

use crate::vm_gobject::VMGObject;
use crate::settings_gobject::SettingsGObject;
use crate::{ADMIN_SERVICE_ADDR, ADMIN_SERVICE_PORT};

pub mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct DataProvider {
        store: Arc<Mutex<ListStore>>,
        settings: Arc<Mutex<SettingsGObject>>,
        pub status: bool,
        admin_client: Arc<RwLock<AdminClient>>,
        service_address: RefCell<(String, u16)>,
        handle: RefCell<Option<JoinHandle<()>>>,
        stop_signal: Arc<AtomicBool>,
    }

    #[derive(Debug)]
    pub enum EventExtended {
        InitialList(Vec<QueryResult>),
        InnerEvent(Event),
        BreakLoop,
    }

    impl DataProvider {
        pub fn new(address: String, port: u16) -> Self {
            let init_store = ListStore::new::<VMGObject>();//Self::fill_by_mock_data();

            Self {
                store: Arc::new(Mutex::new(init_store)),
                settings: Arc::new(Mutex::new(SettingsGObject::default())),
                status: false,
                admin_client: Arc::new(RwLock::new(AdminClient::new(address.clone(), port, None))),
                service_address: RefCell::new((address, port)),
                handle: RefCell::new(None),
                stop_signal: Arc::new(AtomicBool::new(false)),
            }
        }

        pub fn establish_connection(&self) {
            println!("Establishing connection, call watch method... Address: {}:{}", 
            self.service_address.borrow().0, 
            self.service_address.borrow().1);
            let admin_client = self.admin_client.clone();
            let store = self.store.clone();
            let stop_signal = self.stop_signal.clone();
            self.stop_signal.store(false, Ordering::SeqCst);

            let (event_tx, event_rx): (Sender<EventExtended>, Receiver<EventExtended>) = mpsc::channel();

            let handle = thread::spawn(move || {
                Runtime::new().unwrap().block_on(async move {
                    //let Ok(result) = admin_client.watch().await else {println!("Watch call error"); return};

                    //Await with timeout
                    let timeout_duration = Duration::from_secs(5);
                    let Ok(Ok(result)) = timeout(timeout_duration, admin_client.read().unwrap().watch()).await 
                    else {println!("Watch call timeout/error"); return};

                    let list = result.initial.clone();
                    let _ = event_tx.send(EventExtended::InitialList(list));

                    while !(stop_signal.load(Ordering::SeqCst)) {
                        if let Ok(event) = result.channel.try_recv() {
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
                                    if let Some(item) = store_inner.item(i) {
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
                                let store_inner = store.lock().unwrap();
                                for i in 0..store_inner.n_items() {
                                    if let Some(item) = store_inner.item(i) {
                                        let obj = item.downcast_ref::<VMGObject>().unwrap();
                                        if obj.name() == result.name {
                                            obj.update(result);
                                            break;
                                        }
                                    }
                                }
                            },
                            Event::UnitRegistered(result) => {
                                println!("Unit registered {:?}", result);
                                let store_inner = store.lock().unwrap();
                                store_inner.append(&VMGObject::new(result.name, result.description, result.status, result.trust_level));
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
            vec.push(VMGObject::new("VM1".to_string(), String::from("This is the file.pdf and very very long description"), 
                    VMStatus::Running, TrustLevel::NotSecure));
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

        pub fn get_current_service_address(&self) -> (String, u16) {
            self.service_address.clone().into_inner()
        }

        pub fn set_service_address(&self, addr: String, port: u16) {//only when disconnected/watch stopped
            if !(self.admin_client.try_write().is_err()) {
                println!("Set service address {addr}:{port}");
                let mut service_address = self.service_address.borrow_mut();
                *service_address = (addr.clone(), port);
                let mut admin_client = self.admin_client.write().unwrap();
                *admin_client = AdminClient::new(addr, port, None);
            }
        }

        pub fn add_vm(&self, vm: VMGObject) {
            let store = self.store.lock().unwrap();
            store.append(&vm);
        }

        pub fn reconnect(&self, addr: Option<(String, u16)>) {
            println!("Reconnect request...");

            self.disconnect();

            if let Some((host, port)) = addr {
                self.set_service_address(host, port);
            }
            
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
            thread::spawn(move || {//not sure it is really needed
                    Runtime::new().unwrap().block_on(async move {
                    //there is now app name
                    if let Err(error) = admin_client.read().unwrap().start(String::from(""), Some(name), Vec::<String>::new()).await {
                        println!("Start request error {error}");
                    }
                    else {
                        println!("Start request sent");
                    };
                })
            });
        }

        pub fn pause_vm(&self, name: String) {
            let admin_client = self.admin_client.clone();
            thread::spawn(move || {
                Runtime::new().unwrap().block_on(async move {
                    if let Err(error) = admin_client.read().unwrap().pause(name).await {
                        println!("Pause request error {error}");
                    }
                    else {
                        println!("Pause request sent");
                    };
                })
            });
        }

        pub fn resume_vm(&self, name: String) {
            let admin_client = self.admin_client.clone();
            thread::spawn(move || {
                Runtime::new().unwrap().block_on(async move {
                    if let Err(error) = admin_client.read().unwrap().resume(name).await {
                        println!("Resume request error {error}");
                    }
                    else {
                        println!("Resume request sent");
                    };
                })
            });
        }

        pub fn shutdown_vm(&self, name: String) {
            let admin_client = self.admin_client.clone();
            thread::spawn(move || {
                Runtime::new().unwrap().block_on(async move {
                    if let Err(error) = admin_client.read().unwrap().stop(name).await {
                        println!("Stop request error {error}");
                    }
                    else {
                        println!("Stop request sent");
                    };
                })
            });
        }

        pub fn restart_vm(&self, name: String) {
            println!("Restart is not implemented on client lib!");
            //no restart in admin_client
            //self.admin_client.restart(name);
        }

        pub fn add_network(&self, name: String, security: String, password: String) {
            println!("Not yet implemented!");
        }
    }

    impl Default for DataProvider {
        fn default() -> Self {
            Self::new(String::from(ADMIN_SERVICE_ADDR), ADMIN_SERVICE_PORT)
        }
    }

    impl Drop for DataProvider {
        fn drop(&mut self) {
            println!("DataProvider is about to drop");
            self.disconnect();
        }
    }
}

