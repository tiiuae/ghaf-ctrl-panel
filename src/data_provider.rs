use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use gio::ListStore;
use glib::{IsA, Object};
use gtk::{self, gio, glib, prelude::*};

use async_channel::Sender;
use givc_client::{self, AdminClient};
use givc_common::query::{Event, TrustLevel, VMStatus};
use tokio::runtime::Builder;

use crate::vm_gobject::VMGObject;
//use crate::settings_gobject::SettingsGObject;//will be in use in the future

use crate::{ADMIN_SERVICE_ADDR, ADMIN_SERVICE_PORT};

pub mod imp {
    use super::*;

    #[derive(Debug, Clone)]
    struct TypedListStore<T: IsA<Object>>(ListStore, std::marker::PhantomData<T>);

    impl<T: IsA<Object>> TypedListStore<T> {
        pub fn new() -> Self {
            Self(ListStore::new::<T>(), std::marker::PhantomData)
        }

        pub fn get(&self, index: u32) -> Option<T> {
            self.item(index).and_then(|item| item.downcast().ok())
        }

        fn iter(&self) -> impl Iterator<Item = T> {
            let s = self.clone();
            (0..).map_while(move |idx| s.get(idx))
        }
    }

    impl<T: IsA<Object>> Deref for TypedListStore<T> {
        type Target = ListStore;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T: IsA<Object>> DerefMut for TypedListStore<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    type Task = Box<
        dyn for<'a> FnOnce(
                &'a AdminClient,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), String>> + 'a>,
            > + Sync
            + Send,
    >;

    type TaskSender = Sender<(Task, Sender<Result<(), String>>)>;

    #[derive(Debug)]
    pub struct DataProvider {
        store: TypedListStore<VMGObject>,
        //settings: Arc<Mutex<SettingsGObject>>,//will be in use in the future
        pub status: bool,
        service_address: RefCell<(String, u16)>,
        join_handle: RefCell<Option<JoinHandle<()>>>,
        task_runner: Rc<RefCell<Option<Rc<TaskSender>>>>,
    }

    macro_rules! adminclient {
        (|$cl:ident| $block:expr) => {
            Box::new(move |$cl| Box::pin(async move { $block.await.map_err(|e| e.to_string()) }))
        };
    }

    impl DataProvider {
        pub fn new(address: String, port: u16) -> Self {
            let init_store = TypedListStore::new(); //Self::fill_by_mock_data();

            Self {
                store: init_store,
                //settings: Arc::new(Mutex::new(SettingsGObject::default())),
                status: false,
                service_address: RefCell::new((address, port)),
                join_handle: RefCell::new(None),
                task_runner: Rc::new(RefCell::new(None)),
            }
        }

        pub fn establish_connection(&self) {
            println!(
                "Establishing connection, call watch method... Address: {}:{}",
                self.service_address.borrow().0,
                self.service_address.borrow().1
            );

            let (event_tx, event_rx) = async_channel::unbounded();
            let (task_tx, task_rx) =
                async_channel::bounded::<(Task, async_channel::Sender<Result<(), String>>)>(1);
            let (address, port) = self.service_address.borrow().clone();

            let joinhandle = thread::spawn(move || {
                Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async move {
                        let timeout_duration = Duration::from_secs(5);
                        let admin_client = AdminClient::new(address, port, None);
                        let result = {
                            tokio::select! {
                                _ = tokio::time::sleep(timeout_duration) => {
                                    println!("Watch call timeout");
                                    return;
                                },
                                result = admin_client.watch() => match result {
                                    Ok(result) => result,
                                    Err(e) => {
                                        println!("Watch fall failed: {e}");
                                        return;
                                    }
                                },
                                _ = async {
                                    while task_rx.recv().await.is_ok() {
                                        println!("Not yet connected, task ignored");
                                    }
                                } => return,
                            }
                        };
                        println!("Connected!");

                        let _ = event_tx.send((result.channel, result.initial)).await;
                        while let Ok((task, resp)) = task_rx.recv().await {
                            let res = task(&admin_client).await;
                            let _ = resp.send(res.map_err(|e| e.to_string())).await;
                        }
                    });
            });

            let task_tx = Rc::new(task_tx);
            *self.join_handle.borrow_mut() = Some(joinhandle);
            *self.task_runner.borrow_mut() = Some(task_tx.clone());
            let mut store = self.store.clone();
            let task_runner = self.task_runner.clone();
            glib::spawn_future_local(async move {
                let task_weak = Rc::downgrade(&task_tx);
                drop(task_tx);

                if let Ok((channel, initial)) = event_rx.recv().await {
                    store.remove_all();
                    store.extend(initial.into_iter().map(|vm| {
                        VMGObject::new(vm.name, vm.description, vm.status, vm.trust_level)
                    }));

                    while let Ok(event) = channel.recv().await {
                        match event {
                            Event::UnitStatusChanged(result) => {
                                println!("Status: {:?}", result);
                                if let Some(obj) =
                                    store.iter().find(|obj| obj.name() == result.name)
                                {
                                    obj.update(result);
                                }
                            }
                            Event::UnitShutdown(result) => {
                                println!("Shutdown info: {:?}", result);
                                if let Some(obj) =
                                    store.iter().find(|obj| obj.name() == result.name)
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
                }

                if task_weak.upgrade().is_some_and(|rc| {
                    task_runner
                        .borrow()
                        .as_ref()
                        .is_some_and(|tr| Rc::ptr_eq(&rc, tr))
                }) {
                    task_runner.take();
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
            self.store.deref().clone()
        }

        pub fn get_current_service_address(&self) -> (String, u16) {
            self.service_address.borrow().clone()
        }

        pub fn set_service_address(&self, addr: String, port: u16) {
            //wait for stopping
            println!("Set service address {addr}:{port}");
            let mut service_address = self.service_address.borrow_mut();
            *service_address = (addr.clone(), port);
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
            if let Some(joinhandle) = self.join_handle.take() {
                joinhandle.join().unwrap();
                //drop(self.admin_client.clone());
                println!("Client thread stopped!");
            }
        }

        fn client_cmd<F: FnOnce(Result<(), String>) + 'static>(&self, task: Task, cb: F) {
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

        pub fn start_vm(&self, name: String, _vm_name: String) {
            self.client_cmd(
                adminclient!(|client| client.start(name, None /*Some(vm_name)*/, vec![])),
                |res| match res {
                    Ok(_) => println!("Start request sent"),
                    Err(error) => println!("Start request error {error}"),
                },
            );
        }

        pub fn pause_vm(&self, name: String) {
            self.client_cmd(adminclient!(|client| client.pause(name)), |res| match res {
                Ok(_) => println!("Pause request sent"),
                Err(error) => println!("Pause request error {error}"),
            });
        }

        pub fn resume_vm(&self, name: String) {
            self.client_cmd(
                adminclient!(|client| client.resume(name)),
                |res| match res {
                    Ok(_) => println!("Resume request sent"),
                    Err(error) => println!("Resume request error {error}"),
                },
            );
        }

        pub fn shutdown_vm(&self, vm_name: String) {
            self.client_cmd(
                adminclient!(|client| client.stop(vm_name)),
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
                adminclient!(|client| client.set_locale(locale)),
                |res| match res {
                    Ok(_) => println!("Locale set"),
                    Err(e) => println!("Locale setting failed: {e}"),
                },
            );
        }

        pub fn set_timezone(&self, timezone: String) {
            self.client_cmd(
                adminclient!(|client| client.set_timezone(timezone)),
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
