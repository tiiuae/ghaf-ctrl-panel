use gio::ListStore;
use glib::{IsA, Object};
use gtk::{self, gio, glib, prelude::*};
use std::cell::RefCell;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use tokio::runtime::Runtime;
use tokio::time::{sleep, timeout, Duration};

use givc_client::{self, AdminClient};
use givc_common::query::{Event, QueryResult, TrustLevel, VMStatus};

use crate::vm_gobject::VMGObject;
//use crate::settings_gobject::SettingsGObject;//will be in use in the future
use crate::{ADMIN_SERVICE_ADDR, ADMIN_SERVICE_PORT};

pub mod imp {
    use super::*;

    trait TypedStore<T: IsA<Object>> {
        fn get(&self, index: u32) -> Option<T>;
        fn typed_iter(&self) -> impl Iterator<Item = T>;
    }

    impl<T: IsA<Object>> TypedStore<T> for ListStore {
        fn get(&self, index: u32) -> Option<T> {
            self.item(index).and_then(|item| item.downcast().ok())
        }

        fn typed_iter(&self) -> impl Iterator<Item = T> {
            let s = self.clone();
            (0..).map_while(move |idx| s.get(idx))
        }
    }

    #[derive(Debug)]
    pub struct DataProvider {
        store: ListStore,
        //settings: Arc<Mutex<SettingsGObject>>,//will be in use in the future
        pub status: bool,
        admin_client: Arc<RwLock<AdminClient>>,
        service_address: RefCell<(String, u16)>,
        handle: RefCell<Option<JoinHandle<()>>>,
        stop_signal: RefCell<Option<async_channel::Sender<()>>>,
    }

    #[derive(Debug)]
    pub enum EventExtended {
        InitialList(Vec<QueryResult>),
        InnerEvent(Event),
    }

    impl DataProvider {
        pub fn new(address: String, port: u16) -> Self {
            let init_store = ListStore::new::<VMGObject>(); //Self::fill_by_mock_data();

            Self {
                store: init_store,
                //settings: Arc::new(Mutex::new(SettingsGObject::default())),
                status: false,
                admin_client: Arc::new(RwLock::new(AdminClient::new(address.clone(), port, None))),
                service_address: RefCell::new((address, port)),
                handle: RefCell::new(None),
                stop_signal: RefCell::new(None),
            }
        }

        pub fn establish_connection(&self) {
            println!(
                "Establishing connection, call watch method... Address: {}:{}",
                self.service_address.borrow().0,
                self.service_address.borrow().1
            );
            let admin_client = self.admin_client.clone();
            let store = self.store.clone();

            let (event_tx, event_rx) = async_channel::unbounded();
            let (quit_tx, mut quit_rx) = async_channel::bounded::<()>(1);

            let handle = thread::spawn(move || {
                Runtime::new().unwrap().block_on(async move {
                    //let Ok(result) = admin_client.watch().await else {println!("Watch call error"); return};

                    //Await with timeout
                    let timeout_duration = Duration::from_secs(5);
                    let Ok(Ok(result)) =
                        timeout(timeout_duration, admin_client.read().unwrap().watch()).await
                    else {
                        println!("Watch call timeout/error");
                        return;
                    };

                    let list = result.initial.clone();
                    let _ = event_tx.send(EventExtended::InitialList(list));

                    loop {
                        tokio::select! {
                            _ = quit_rx.recv() => break,
                            event = result.channel.recv() => {
                                println!("Got message: {event:?}");
                                if let Ok(event) = event {
                                    let _ = event_tx.send(EventExtended::InnerEvent(event)).await;
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                });
            });

            *self.handle.borrow_mut() = Some(handle);

            let store = self.store.clone();
            glib::spawn_future_local(async move {
                while let Ok(event_ext) = event_rx.recv().await {
                    match event_ext {
                        EventExtended::InitialList(list) => {
                            println!("Initial list: {:?}", list);
                            store.remove_all();
                            for vm in list {
                                store_inner.append(&VMGObject::new(
                                    vm.name,
                                    vm.description,
                                    vm.status,
                                    vm.trust_level,
                                ));
                            }
                        }
                        EventExtended::InnerEvent(event) => match event {
                            Event::UnitStatusChanged(result) => {
                                println!("Status: {:?}", result);
                                if let Some(obj) = store
                                    .typed_iter()
                                    .find(|obj: &VMGObject| obj.name() == result.name)
                                {
                                    obj.update(result);
                                }
                            }
                            Event::UnitShutdown(result) => {
                                println!("Shutdown info: {:?}", result);
                                if let Some(obj) = store
                                    .typed_iter()
                                    .find(|obj: &VMGObject| obj.name() == result.name)
                                {
                                    obj.update(result);
                                }
                            }
                            Event::UnitRegistered(result) => {
                                println!("Unit registered {:?}", result);
                                let store_inner = store.lock().unwrap();
                                store_inner.append(&VMGObject::new(
                                    result.name,
                                    result.description,
                                    result.status,
                                    result.trust_level,
                                ));
                            }
                        },
                    }
                }
            });
        }

        fn fill_by_mock_data() -> ListStore {
            let init_store = ListStore::new::<VMGObject>();
            let mut vec: Vec<VMGObject> = Vec::new();
            vec.push(VMGObject::new(
                "VM1".to_string(),
                String::from("This is the file.pdf and very very long description"),
                VMStatus::Running,
                TrustLevel::NotSecure,
            ));
            vec.push(VMGObject::new(
                "VM2".to_string(),
                String::from("Google Chrome"),
                VMStatus::Paused,
                TrustLevel::Secure,
            ));
            vec.push(VMGObject::new(
                "VM3".to_string(),
                String::from("AppFlowy"),
                VMStatus::Running,
                TrustLevel::Secure,
            ));
            init_store.extend_from_slice(&vec);
            return init_store;
        }

        pub fn get_store(&self) -> ListStore {
            self.store.clone()
        }

        pub fn get_current_service_address(&self) -> (String, u16) {
            self.service_address.clone().into_inner()
        }

        pub fn set_service_address(&self, addr: String, port: u16) {
            //wait for stopping
            while (self.admin_client.try_write().is_err()) {}

            println!("Set service address {addr}:{port}");
            let mut service_address = self.service_address.borrow_mut();
            *service_address = (addr.clone(), port);
            let mut admin_client = self.admin_client.write().unwrap();
            *admin_client = AdminClient::new(addr, port, None);
        }

        pub fn add_vm(&self, vm: VMGObject) {
            self.store.append(&vm);
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
            self.stop_signal.take();
            // self.stop_signal.store(true, Ordering::SeqCst);
            if let Some(handle) = self.handle.take() {
                handle.join().unwrap();
                //drop(self.admin_client.clone());
                println!("Client thread stopped!");
            }
        }

        pub fn start_vm(&self, name: String, _vm_name: String) {
            //need to test
            let admin_client = self.admin_client.clone();
            thread::spawn(move || {
                //not sure it is really needed
                Runtime::new().unwrap().block_on(async move {
                    if let Err(error) = admin_client
                        .read()
                        .unwrap()
                        .start(name, None /*Some(vm_name)*/, Vec::<String>::new())
                        .await
                    {
                        println!("Start request error {error}");
                    } else {
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
                    } else {
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
                    } else {
                        println!("Resume request sent");
                    };
                })
            });
        }

        pub fn shutdown_vm(&self, vm_name: String) {
            let admin_client = self.admin_client.clone();
            thread::spawn(move || {
                Runtime::new().unwrap().block_on(async move {
                    if let Err(error) = admin_client.read().unwrap().stop(vm_name).await {
                        println!("Stop request error {error}");
                    } else {
                        println!("Stop request sent");
                    };
                })
            });
        }

        pub fn set_locale(&self, locale: String) {
            let admin_client = self.admin_client.clone();
            thread::spawn(move || {
                Runtime::new().unwrap().block_on(async move {
                    if let Err(error) = admin_client.read().unwrap().set_locale(locale).await {
                        println!("Locale request error {error}");
                    } else {
                        println!("Locale request sent");
                    };
                })
            });
        }

        pub fn set_timezone(&self, timezone: String) {
            let admin_client = self.admin_client.clone();
            thread::spawn(move || {
                Runtime::new().unwrap().block_on(async move {
                    if let Err(error) = admin_client.read().unwrap().set_timezone(timezone).await {
                        println!("Timezone request error {error}");
                    } else {
                        println!("Timezone request sent");
                    };
                })
            });
        }

        pub fn restart_vm(&self, name: String) {
            println!("Restart is not implemented on client lib!");
            //no restart in admin_client
            //self.admin_client.restart(name);
        }

        pub fn add_network(&self, _name: String, _security: String, _password: String) {
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
