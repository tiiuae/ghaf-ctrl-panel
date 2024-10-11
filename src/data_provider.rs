use async_channel::Sender;
use gio::ListStore;
use glib::{IsA, Object};
use gtk::{self, gio, glib, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::thread::{self, JoinHandle};
use tokio::runtime::{Builder, Handle};
use tokio::time::{timeout, Duration};

use givc_client::{self, AdminClient};
use givc_common::query::{Event, TrustLevel, VMStatus};
//use givc_client::endpoint::{EndpointConfig, TlsConfig};
//use givc_common::types::*;

use crate::vm_gobject::VMGObject;

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

    type Task = Box<
        dyn for<'a> FnOnce(
                RwLockReadGuard<'a, AdminClient>,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), String>> + 'a>,
            > + Sync
            + Send,
    >;

    #[derive(Debug)]
    pub struct DataProvider {
        store: ListStore,
        //settings: Arc<Mutex<SettingsGObject>>,//will be in use in the future
        pub status: bool,
        admin_client: Arc<RwLock<AdminClient>>,
        service_address: RefCell<(String, u16)>,
        handle: Rc<RefCell<Option<Handle>>>,
        join_handle: RefCell<Option<JoinHandle<()>>>,
        //task_runner: RefCell<Option<Sender<(Task, Sender<Result<(), String>>)>>>,
        task_runner: Rc<RefCell<Option<Sender<(Task, Sender<Result<(), String>>)>>>>,
    }

    macro_rules! adminclient {
        (|$cl:ident| $block:expr) => {
            Box::new(move |$cl| {
                Box::pin(async move { $block.map_err(|e| e.to_string()) })
            })
        }
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
                join_handle: RefCell::new(None),
                handle: Rc::new(RefCell::new(None)),
                task_runner: Rc::new(RefCell::new(None)),
            }
        }

        pub fn establish_connection(&self) {
            println!(
                "Establishing connection, call watch method... Address: {}:{}",
                self.service_address.borrow().0,
                self.service_address.borrow().1
            );
            let admin_client = self.admin_client.clone();
            let _store = self.store.clone();

            let (event_tx, event_rx) = async_channel::unbounded();
            let (task_tx, task_rx) =
                async_channel::bounded::<(Task, async_channel::Sender<Result<(), String>>)>(1);

            let joinhandle = thread::spawn(move || {
                Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async move {
                        let timeout_duration = Duration::from_secs(5);
                        let Ok(Ok(result)) =
                            timeout(timeout_duration, admin_client.read().unwrap().watch()).await
                        else {
                            println!("Watch call timeout/error");
                            return;
                        };
                        println!("Connected!");

                        let _ = event_tx
                            .send((result.channel, result.initial, Handle::current().clone()))
                            .await;
                        while let Ok((task, resp)) = task_rx.recv().await {
                            let admin_client = admin_client.read().unwrap();
                            let res = task(admin_client).await;
                            let _ = resp.send(res.map_err(|e| e.to_string())).await;
                        }
                    });
            });

            *self.join_handle.borrow_mut() = Some(joinhandle);
            let store = self.store.clone();
            let selfhandle = self.handle.clone();
            let taskrunner = self.task_runner.clone();
            glib::spawn_future_local(async move {
                let Ok((channel, initial, handle)) = event_rx.recv().await else {
                    return;
                };
                println!("Got stuff!");
                *selfhandle.borrow_mut() = Some(handle);
                *taskrunner.borrow_mut() = Some(task_tx);
                store.remove_all();
                for vm in initial {
                    store.append(&VMGObject::new(
                        vm.name,
                        vm.description,
                        vm.status,
                        vm.trust_level,
                    ));
                }

                while let Ok(event) = channel.recv().await {
                    match event {
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
                            store.append(&VMGObject::new(
                                result.name,
                                result.description,
                                result.status,
                                result.trust_level,
                            ));
                        }
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
            init_store
        }

        pub fn get_store(&self) -> ListStore {
            self.store.clone()
        }

        pub fn get_current_service_address(&self) -> (String, u16) {
            self.service_address.clone().into_inner()
        }

        pub fn set_service_address(&self, addr: String, port: u16) {
            //only when disconnected/watch stopped
            if self.admin_client.try_write().is_ok() {
                println!("Set service address {addr}:{port}");
                let mut service_address = self.service_address.borrow_mut();
                *service_address = (addr.clone(), port);
                let mut admin_client = self.admin_client.write().unwrap();
                *admin_client = AdminClient::new(addr, port, None);
            }
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
            self.task_runner.take();
            self.handle.take();
            if let Some(joinhandle) = self.join_handle.take() {
                joinhandle.join().unwrap();
                //drop(self.admin_client.clone());
                println!("Client thread stopped!");
            }
        }

        fn client_cmd<F: FnOnce(Result<(), String>) -> () + 'static>(&self, task: Task, cb: F) {
            let (res_tx, res_rx) = async_channel::bounded(1);
            let Some(tr) = self.task_runner.borrow().as_ref().cloned() else {
                cb(Err("Not connected to admin".into()));
                return;
            };
            glib::spawn_future_local(async move {
                // On error res_tx is dropped and res_rx.recv() will fail below
                let _ = tr.send((task, res_tx)).await;
                let res = match res_rx.recv().await {
                    Ok(res) => res,
                    Err(err) => Err(err.to_string()),
                };
                cb(res);
            });
        }
        
        pub fn start_vm(&self, name: String) {
            self.client_cmd(
                adminclient!(|client| client.start("".to_owned(), Some(name), vec![]).await),
                |res| match res {
                    Ok(_) => println!("Start request sent"),
                    Err(error) => println!("Start request error {error}"),
                },
            );
        }

        pub fn pause_vm(&self, name: String) {
            self.client_cmd(
                adminclient!(|client| client.pause(name).await),
                |res| match res {
                    Ok(_) => println!("Pause request sent"),
                    Err(error) => println!("Pause request error {error}"),
                },
            );
        }

        pub fn resume_vm(&self, name: String) {
            self.client_cmd(
                adminclient!(|client| client.resume(name).await),
                |res| match res {
                    Ok(_) => println!("Resume request sent"),
                    Err(error) => println!("Resume request error {error}"),
                },
            );
        }

        pub fn shutdown_vm(&self, name: String) {
            self.client_cmd(
                adminclient!(|client| client.stop(name).await),
                |res| match res {
                    Ok(_) => println!("Stop request sent"),
                    Err(error) => println!("Stop request error {error}"),
                },
            );
        }

        pub fn restart_vm(&self, _name: String) {
            println!("Restart is not implemented on client lib!");
            //no restart in admin_client
            //self.admin_client.restart(name);
        }

        pub fn set_locale(&self, locale: String) {
            self.client_cmd(
                adminclient!(|client| client.set_locale(locale).await),
                |res| match res {
                    Ok(_) => println!("Locale set"),
                    Err(e) => println!("Locale setting failed: {e}"),
                },
            );
        }

        pub fn set_timezone(&self, timezone: String) {
            self.client_cmd(
                adminclient!(|client| client.set_timezone(timezone).await),
                |res| match res {
                    Ok(_) => println!("Timezone set"),
                    Err(e) => println!("Timezone setting failed: {e}"),
                },
            );
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
