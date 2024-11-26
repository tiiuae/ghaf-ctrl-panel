use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use gio::ListStore;
use glib::Object;
use gtk::{self, gio, glib, prelude::*};

use async_channel::Sender;
use givc_client::endpoint::TlsConfig;
use givc_client::{self, AdminClient};
use givc_common::query::{Event, TrustLevel, VMStatus};
use givc_common::types::ServiceType;
use tokio::runtime::Builder;

use crate::data_gobject::DataGObject;
use crate::service_gobject::ServiceGObject;
//use crate::settings_gobject::SettingsGObject;//will be in use in the future

use crate::{ADMIN_SERVICE_ADDR, ADMIN_SERVICE_PORT};

pub mod imp {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct TypedListStore<T: IsA<Object>>(ListStore, std::marker::PhantomData<T>);

    pub struct LanguageRegionEntry {
        pub code: String,
        pub display: String,
    }

    impl<T: Into<String>, U: Into<String>> From<(T, U)> for LanguageRegionEntry {
        fn from(val: (T, U)) -> Self {
            Self {
                code: val.0.into(),
                display: val.1.into(),
            }
        }
    }

    impl From<LanguageRegionEntry> for DataGObject {
        fn from(val: LanguageRegionEntry) -> Self {
            Self::new(val.code, val.display)
        }
    }

    pub struct LanguageRegionData {
        pub timezones: Vec<LanguageRegionEntry>,
        pub current_timezone: Option<String>,
        pub languages: Vec<LanguageRegionEntry>,
        pub current_language: Option<String>,
    }

    impl<T: IsA<Object>> TypedListStore<T> {
        pub fn new() -> Self {
            Self(ListStore::new::<T>(), std::marker::PhantomData)
        }

        pub fn get(&self, index: u32) -> Option<T> {
            self.item(index).and_then(|item| item.downcast().ok())
        }

        pub fn iter(&self) -> impl Iterator<Item = T> {
            let s = self.clone();
            (0..).map_while(move |idx| s.get(idx))
        }
    }

    impl<T: IsA<Object>, L: IsA<ListStore>> From<L> for TypedListStore<T> {
        fn from(store: L) -> Self {
            Self(store.upcast(), std::marker::PhantomData)
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
        store: TypedListStore<ServiceGObject>,
        //settings: Arc<Mutex<SettingsGObject>>,//will be in use in the future
        pub status: bool,
        service_address: RefCell<(String, u16)>,
        tls_info: RefCell<Option<(String, TlsConfig)>>,
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
            let init_store = Self::fill_by_mock_data(); //ListStore::new::<ServiceGObject>();

            Self {
                store: init_store.into(),
                //settings: Arc::new(Mutex::new(SettingsGObject::default())),
                status: false,
                service_address: RefCell::new((address, port)),
                tls_info: RefCell::new(None),
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
            let tls_info = self.tls_info.borrow().clone();

            let joinhandle = thread::spawn(move || {
                Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async move {
                        let timeout_duration = Duration::from_secs(5);
                        let admin_client = AdminClient::new(address, port, tls_info);
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
                    store.extend(initial.into_iter().map(|service| {
                        ServiceGObject::new(
                            service.name,
                            service.description,
                            service.status,
                            service.trust_level,
                            service.service_type,
                            service.vm_name,
                        )
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
                                //Remove service/app, update VM
                                if let Some(pos) =
                                    store.iter().position(|obj| obj.name() == result.name)
                                {
                                    let obj: ServiceGObject = store.get(pos as u32).unwrap();

                                    if obj.is_vm() {
                                        obj.update(result);
                                    } else {
                                        store.remove(pos as u32);
                                    }
                                }
                            }
                            Event::UnitRegistered(result) => {
                                println!("Unit registered {:?}", result);
                                store.append(&ServiceGObject::new(
                                    result.name,
                                    result.description,
                                    result.status,
                                    result.trust_level,
                                    result.service_type,
                                    result.vm_name,
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
            let init_store = ListStore::new::<ServiceGObject>();
            let mut vec: Vec<ServiceGObject> = Vec::new();
            vec.push(ServiceGObject::new(
                "microvm@zathura-vm.service".to_string(),
                String::from("This is the file.pdf and very very long description"),
                VMStatus::Running,
                TrustLevel::NotSecure,
                ServiceType::VM,
                None,
            ));
            vec.push(ServiceGObject::new(
                "chrome@1.service".to_string(),
                String::from("Google Chrome"),
                VMStatus::Paused,
                TrustLevel::Secure,
                ServiceType::App,
                Some(String::from("TestVM")),
            ));
            vec.push(ServiceGObject::new(
                "appflowy@1.service".to_string(),
                String::from("AppFlowy"),
                VMStatus::Running,
                TrustLevel::Secure,
                ServiceType::Svc,
                None,
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

        pub fn set_tls_info(&self, value: Option<(String, TlsConfig)>) {
            let mut tls_info = self.tls_info.borrow_mut();
            *tls_info = value;
        }

        pub fn add_vm(&self, vm: ServiceGObject) {
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

        async fn client_cmd_async(&self, task: Task) -> Result<(), String> {
            let (res_tx, res_rx) = async_channel::bounded(1);
            let Some(tr) = self.task_runner.borrow().as_ref().cloned() else {
                return Err("Not connected to admin".into());
            };
            // On error res_tx is dropped and res_rx.recv() will fail below
            let _ = tr.send((task, res_tx)).await;
            match res_rx.recv().await {
                Ok(res) => res,
                Err(err) => Err(err.to_string()),
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

        pub fn start_service(&self, name: String) {
            let store = self.store.clone();
            if let Some(obj) = store.iter().find(|obj| obj.name() == name) {
                if obj.is_app() {
                    let app_name = obj.display_name();
                    self.client_cmd(
                        adminclient!(|client| client.start(
                            app_name,
                            None, /*Some(vm_name)*/
                            vec![]
                        )),
                        |res| match res {
                            Ok(_) => println!("Start app request sent"),
                            Err(error) => println!("Start app request error {error}"),
                        },
                    );
                } else {
                    //another function for VMs and services?
                }
            }
        }

        pub fn pause_service(&self, name: String) {
            self.client_cmd(adminclient!(|client| client.pause(name)), |res| match res {
                Ok(_) => println!("Pause request sent"),
                Err(error) => println!("Pause request error {error}"),
            });
        }

        pub fn resume_service(&self, name: String) {
            self.client_cmd(
                adminclient!(|client| client.resume(name)),
                |res| match res {
                    Ok(_) => println!("Resume request sent"),
                    Err(error) => println!("Resume request error {error}"),
                },
            );
        }

        pub fn stop_service(&self, name: String) {
            let store = self.store.clone();
            if let Some(obj) = store.iter().find(|obj| obj.name() == name) {
                let mut name_to_use = name;
                if obj.is_app() {
                    name_to_use = obj.display_name();
                }
                self.client_cmd(
                    adminclient!(|client| client.stop(name_to_use)),
                    |res| match res {
                        Ok(_) => println!("Stop request sent"),
                        Err(error) => println!("Stop request error {error}"),
                    },
                );
            }
        }

        pub fn restart_service(&self, _name: String) {
            println!("Restart is not implemented on client lib!");
            //no restart in admin_client
            //self.admin_client.restart(name);
        }

        pub async fn set_locale(&self, locale: String) -> Result<(), String> {
            self.client_cmd_async(adminclient!(|client| client.set_locale(locale)))
                .await
        }

        pub async fn set_timezone(&self, timezone: String) -> Result<(), String> {
            self.client_cmd_async(adminclient!(|client| client.set_timezone(timezone)))
                .await
        }

        pub fn add_network(&self, _name: String, _security: String, _password: String) {
            println!("Not yet implemented!");
        }

        fn path_join<P: AsRef<std::path::Path>, Pa: AsRef<std::path::Path>>(
            base: P,
            rel: Pa,
        ) -> std::path::PathBuf {
            use std::path::Component::*;
            base.as_ref()
                .components()
                .chain(rel.as_ref().components())
                .fold(Vec::new(), |mut path, part| {
                    match part {
                        Prefix(_) => {
                            path.clear();
                            path.push(part);
                        }
                        RootDir => {
                            path.retain(|p| matches!(p, Prefix(_)));
                            path.push(part);
                        }
                        CurDir => {}
                        ParentDir => {
                            if !matches!(path.last(), Some(Prefix(_) | RootDir)) {
                                path.pop();
                            }
                        }
                        part => path.push(part),
                    }
                    path
                })
                .into_iter()
                .collect()
        }

        fn get_current_locale() -> Result<String, Box<dyn std::error::Error>> {
            String::from_utf8(std::fs::read("/etc/locale.conf")?)?
                .lines()
                .find_map(|line| {
                    line.split_once('=')
                        .and_then(|(var, val)| (var == "LANG").then(|| val.to_owned()))
                })
                .ok_or_else(|| "LANG not found".into())
                .map(|s| {
                    s.split_once('.')
                        .map(|(loc, cset)| {
                            loc.chars()
                                .chain(std::iter::once('.'))
                                .chain(cset.chars().filter_map(|c| match c {
                                    c if c.is_ascii_alphanumeric() => Some(c.to_ascii_lowercase()),
                                    _ => None,
                                }))
                                .collect::<String>()
                        })
                        .unwrap_or(s)
                })
        }

        fn get_current_timezone() -> Result<String, Box<dyn std::error::Error>> {
            use std::path::{Component::*, Path, PathBuf};
            let p = Self::path_join("/etc", std::fs::read_link("/etc/localtime")?);
            p.strip_prefix("/etc/zoneinfo")?
                .to_str()
                .ok_or_else(|| "Invalid characters in timezone".into())
                .map(ToOwned::to_owned)
        }

        fn get_locales() -> Result<Vec<LanguageRegionEntry>, Box<dyn std::error::Error>> {
            let output = Command::new("locale").arg("-va").output()?;
            let mut locale = None;
            let mut lang = None;
            let mut terr = None;
            let mut locales = Vec::new();

            for line in String::from_utf8(output.stdout)?
                .lines()
                .map(str::trim)
                .chain(std::iter::once(""))
            {
                if line.is_empty() {
                    if let Some((locale, lang, terr)) = locale
                        .take()
                        .map(|loc: String| (loc, lang.take(), terr.take()))
                    {
                        if locale
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_lowercase())
                        {
                            let lang = lang
                                .map(|lang| {
                                    if let Some(terr) = terr {
                                        format!("{lang} ({terr})")
                                    } else {
                                        lang
                                    }
                                })
                                .unwrap_or_else(|| locale.clone());
                            locales.push(LanguageRegionEntry {
                                code: locale,
                                display: lang,
                            });
                        }
                    }
                }
                if let Some(loc) = line
                    .strip_prefix("locale: ")
                    .and_then(|l| l.split_once(' ').map(|(a, _)| a))
                {
                    locale = Some(loc.to_owned());
                } else if let Some(lan) = line.strip_prefix("language | ") {
                    lang = Some(lan.to_owned());
                } else if let Some(ter) = line.strip_prefix("territory | ") {
                    terr = Some(ter.to_owned());
                }
            }

            Ok(locales)
        }

        pub fn get_timezone_display(tz: &str) -> String {
            tz.chars().map(|c| if c == '_' { ' ' } else { c }).collect()
        }

        pub fn get_timezones() -> Result<Vec<LanguageRegionEntry>, Box<dyn std::error::Error>> {
            let output = Command::new("timedatectl").arg("list-timezones").output()?;
            Ok(String::from_utf8(output.stdout)?
                .lines()
                .map(|tz| (tz, Self::get_timezone_display(tz)).into())
                .collect())
        }

        pub async fn get_timezone_locale_info() -> LanguageRegionData {
            let (tx_lang, rx_lang) = async_channel::bounded(1);
            let (tx_tz, rx_tz) = async_channel::bounded(1);
            std::thread::spawn(move || {
                if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let current = match DataProvider::get_current_locale() {
                        Ok(v) => Some(v),
                        Err(e) => {
                            println!("Error detecting current locale: {e}");
                            Some(String::from("en_US.utf8"))
                        }
                    };
                    let locales = DataProvider::get_locales()?;
                    Ok(tx_lang.send_blocking((current, locales))?)
                })() {
                    println!("Getting locales failed: {e}, using defaults");
                    drop(tx_lang);
                }

                if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let current = match DataProvider::get_current_timezone() {
                        Ok(v) => Some(v),
                        Err(e) => {
                            println!("Error detecting current timezone: {e}");
                            Some(String::from("UTC"))
                        }
                    };
                    let timezones = DataProvider::get_timezones()?;
                    Ok(tx_tz.send_blocking((current, timezones))?)
                })() {
                    println!("Getting timezones failed: {e}, using defaults");
                }
            });

            let (current_language, languages) = rx_lang.recv().await.unwrap_or_else(|_| {
                (
                    Some(String::from("en_US.utf8")),
                    vec![
                        ("ar_AE.utf8", "Arabic (UAE)").into(),
                        ("en_US.utf8", "English (United States)").into(),
                    ],
                )
            });

            let (current_timezone, timezones) = rx_tz.recv().await.unwrap_or_else(|_| {
                (
                    Some(String::from("UTC")),
                    vec![
                        ("Europe/Helsinki", "Europe/Helsinki").into(),
                        ("Asia/Abu_Dhabi", "Asia/Abu Dhabi").into(),
                    ],
                )
            });

            LanguageRegionData {
                languages,
                current_language,
                timezones,
                current_timezone,
            }
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
