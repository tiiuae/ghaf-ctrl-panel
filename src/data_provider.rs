use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use gio::{ListModel, ListStore};
use gtk::{self, gio, glib};

use async_channel::Sender;
pub use givc_client::client::StatsResponse;
use givc_client::endpoint::TlsConfig;
use givc_client::{self, AdminClient};
use givc_common::address::EndpointAddress;
use givc_common::pb::admin::StartResponse;
use givc_common::query::{Event, TrustLevel, VMStatus};
use givc_common::types::ServiceType;
use tokio::runtime::Builder;

use crate::data_gobject::DataGObject;
use crate::service_gobject::ServiceGObject;
use crate::typed_list_store::imp::{TypedListStore, TypedModelExt};
//use crate::settings_gobject::SettingsGObject;//will be in use in the future

use crate::{ADMIN_SERVICE_ADDR, ADMIN_SERVICE_PORT};

pub mod imp {
    use super::*;

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

    #[derive(Debug, Clone)]
    enum Response {
        Empty,
        Stats(StatsResponse),
        Start(StartResponse),
        Error(String),
    }

    impl std::convert::TryFrom<Response> for StatsResponse {
        type Error = String;

        fn try_from(r: Response) -> Result<StatsResponse, Self::Error> {
            use Response::*;
            match r {
                Stats(r) => Ok(r),
                Error(e) => Err(e),
                _ => Err("Unexpected response".into()),
            }
        }
    }

    impl std::convert::TryFrom<Response> for StartResponse {
        type Error = String;

        fn try_from(r: Response) -> Result<StartResponse, Self::Error> {
            use Response::*;
            match r {
                Start(r) => Ok(r),
                Error(e) => Err(e),
                _ => Err("Unexpected response".into()),
            }
        }
    }

    impl<E: ToString> From<Result<StatsResponse, E>> for Response {
        fn from(r: Result<StatsResponse, E>) -> Response {
            use Response::*;
            match r {
                Ok(r) => Stats(r),
                Err(e) => Error(e.to_string()),
            }
        }
    }

    impl<E: ToString> From<Result<StartResponse, E>> for Response {
        fn from(r: Result<StartResponse, E>) -> Response {
            use Response::*;
            match r {
                Ok(r) => Start(r),
                Err(e) => Error(e.to_string()),
            }
        }
    }

    impl<E: ToString> From<Result<(), E>> for Response {
        fn from(r: Result<(), E>) -> Response {
            use Response::*;
            match r {
                Ok(()) => Empty,
                Err(e) => Error(e.to_string()),
            }
        }
    }

    impl std::convert::TryFrom<Response> for () {
        type Error = String;

        fn try_from(r: Response) -> Result<(), Self::Error> {
            use Response::*;
            match r {
                Empty => Ok(()),
                Error(e) => Err(e),
                _ => Err("Unexpected response".into()),
            }
        }
    }

    type Task = Box<
        dyn for<'a> FnOnce(
                &'a AdminClient,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + 'a>>
            + Sync
            + Send,
    >;

    type TaskSender = Sender<(Task, Sender<Response>)>;

    #[derive(Debug)]
    pub struct DataProvider {
        store: TypedListStore<ServiceGObject>,
        pub status: bool,
        service_address: RefCell<EndpointAddress>,
        tls_info: RefCell<Option<(String, TlsConfig)>>,
        join_handle: RefCell<Option<JoinHandle<()>>>,
        task_runner: Rc<RefCell<Option<Rc<TaskSender>>>>,
    }

    macro_rules! adminclient {
        (|$cl:ident| $block:expr) => {
            Box::new(move |$cl| Box::pin(async move { $block.await.into() }))
        };
    }

    impl DataProvider {
        pub fn new(addr: String, port: u16) -> Self {
            let init_store = Self::fill_by_mock_data(); //ListStore::new::<ServiceGObject>();

            Self {
                store: init_store.into(),
                //settings: Arc::new(Mutex::new(SettingsGObject::default())),
                status: false,
                service_address: RefCell::new(EndpointAddress::Tcp { addr, port }),
                tls_info: RefCell::new(None),
                join_handle: RefCell::new(None),
                task_runner: Rc::new(RefCell::new(None)),
            }
        }

        pub fn establish_connection(&self) {
            println!(
                "Establishing connection, call watch method... Address: {:?}",
                self.service_address.borrow(),
            );

            let (event_tx, event_rx) = async_channel::unbounded();
            let (task_tx, task_rx) =
                async_channel::bounded::<(Task, async_channel::Sender<Response>)>(1);
            let address = self.service_address.borrow().clone();
            let tls_info = self.tls_info.borrow().clone();

            let joinhandle = thread::spawn(move || {
                Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async move {
                        let timeout_duration = Duration::from_secs(5);
                        let admin_client = AdminClient::from_endpoint_address(address, tls_info);
                        let result = {
                            tokio::select! {
                                _ = tokio::time::sleep(timeout_duration) => {
                                    println!("Watch call timeout");
                                    return;
                                },
                                result = admin_client.watch() => match result {
                                    Ok(result) => result,
                                    Err(e) => {
                                        println!("Watch call failed: {e}");
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
                            let _ = resp.send(res).await;
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
                    println!("Initial list {:?}", initial);
                    store.extend(initial.into_iter().map(Into::into));

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
                                if let Some((pos, obj)) = store
                                    .iter()
                                    .enumerate()
                                    .find(|(_, obj)| obj.name() == result.name)
                                {
                                    if obj.is_vm() {
                                        obj.update(result);
                                    } else {
                                        store.remove(pos as u32);
                                    }
                                }
                            }
                            Event::UnitRegistered(result) => {
                                println!("Unit registered {:?}", result);
                                store.extend(Some(result.into()));
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
            [
                ServiceGObject::new(
                    "microvm@zathura-vm.service".to_string(),
                    String::from("This is the file.pdf and very very long description"),
                    VMStatus::Running,
                    TrustLevel::NotSecure,
                    ServiceType::VM,
                    Some(String::from("zathura-vm")),
                ),
                ServiceGObject::new(
                    "chrome@1.service".to_string(),
                    String::from("Google Chrome"),
                    VMStatus::Paused,
                    TrustLevel::Secure,
                    ServiceType::App,
                    Some(String::from("TestVM")),
                ),
                ServiceGObject::new(
                    "appflowy@1.service".to_string(),
                    String::from("AppFlowy"),
                    VMStatus::Running,
                    TrustLevel::Secure,
                    ServiceType::Svc,
                    None,
                ),
            ]
            .into_iter()
            .collect::<ListStore>()
        }

        pub fn get_store(&self) -> ListModel {
            self.store.model()
        }

        pub fn get_current_service_address(&self) -> EndpointAddress {
            self.service_address.borrow().clone()
        }

        pub fn set_service_address(&self, addr: EndpointAddress) {
            //wait for stopping
            println!("Set service address {addr:?}");
            let mut service_address = self.service_address.borrow_mut();
            *service_address = addr;
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

            if let Some((addr, port)) = addr {
                self.set_service_address(EndpointAddress::Tcp { addr, port });
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

        fn client_cmd_async<T: std::convert::TryFrom<Response, Error = String>>(
            &self,
            task: Task,
        ) -> impl std::future::Future<Output = Result<T, String>> {
            let (res_tx, res_rx) = async_channel::bounded(1);
            let tr = self.task_runner.borrow().as_ref().cloned();

            async move {
                let Some(tr) = tr else {
                    return Err("Not connected to admin".into());
                };
                let _ = tr.send((task, res_tx)).await;
                // On error res_tx is dropped and res_rx.recv() will fail below
                res_rx.recv().await.map_err(|e| e.to_string())?.try_into()
            }
        }

        fn client_cmd<R, F>(&self, task: Task, cb: F)
        where
            R: std::convert::TryFrom<Response, Error = String>,
            F: FnOnce(Result<R, String>) + 'static,
        {
            let (res_tx, res_rx) = async_channel::bounded(1);
            let Some(tr) = self.task_runner.borrow().as_ref().cloned() else {
                cb(Err("Not connected to admin".into()));
                return;
            };
            glib::spawn_future_local(async move {
                // On error res_tx is dropped and res_rx.recv() will fail below
                let _ = tr.send((task, res_tx)).await;
                let res = match res_rx.recv().await {
                    Ok(res) => res.try_into(),
                    Err(err) => Err(err.to_string()),
                };
                cb(res);
            });
        }

        pub fn start_service(&self, name: String) {
            let store = self.store.clone();
            let Some(obj) = store.iter().find(|obj| obj.name() == name) else {
                return;
            };
            if obj.is_vm() {
                let name_clone = name.clone();
                self.client_cmd(
                    adminclient!(|client| client.start_vm(name)),
                    move |res| match res {
                        Ok::<StartResponse, _>(_) => {
                            println!("Start VM {name_clone} request sent")
                        }
                        Err(error) => {
                            println!("Start VM {name_clone} request error {error}")
                        }
                    },
                )
                //basicaly, there is no need to start app or service
            } else if obj.is_app() {
                let app_name = obj.display_name(); //not sure
                let vm_name = obj.vm_name(); //if it is known
                self.client_cmd(
                    adminclient!(|client| client.start_app(app_name, vm_name, vec![])),
                    |res| match res {
                        Ok::<StartResponse, _>(_) => println!("Start app request sent"),
                        Err(error) => println!("Start app request error {error}"),
                    },
                )
            } else {
                let name_clone = name.clone();
                let vm_name = obj.vm_name(); //if it is known
                self.client_cmd(
                    adminclient!(|client| client.start_service(name, vm_name)),
                    move |res| match res {
                        Ok::<StartResponse, _>(_) => {
                            println!("Start service {name_clone} request sent")
                        }
                        Err(error) => {
                            println!("Start service {name_clone} request error {error}")
                        }
                    },
                )
            }
        }

        pub fn start_app_in_vm(&self, app: String, vm: String, args: Vec<String>) {
            let app_name = app.clone();
            let vm_name = vm.clone();
            self.client_cmd(
                adminclient!(|client| client.start_app(app, vm, args)),
                move |res| match res {
                    Ok::<StartResponse, _>(_) => {
                        println!("Start app {app_name} in the VM {vm_name} request sent")
                    }
                    Err(error) => {
                        println!("Start app {app_name} in VM {vm_name} request error {error}")
                    }
                },
            );
        }

        pub fn pause_service(&self, name: String) {
            self.client_cmd(adminclient!(|client| client.pause(name)), |res| match res {
                Ok(()) => println!("Pause request sent"),
                Err(error) => println!("Pause request error {error}"),
            });
        }

        pub fn resume_service(&self, name: String) {
            self.client_cmd(
                adminclient!(|client| client.resume(name)),
                |res: Result<StartResponse, _>| match res {
                    Ok(_) => println!("Resume request sent"),
                    Err(error) => println!("Resume request error {error}"),
                },
            );
        }

        pub fn stop_service(&self, name: String) {
            self.client_cmd(adminclient!(|client| client.stop(name)), |res| match res {
                Ok(()) => println!("Stop request sent"),
                Err(error) => println!("Stop request error {error}"),
            });
        }

        pub fn restart_service(&self, _name: String) {
            println!("Restart is not implemented on client lib!");
            //no restart in admin_client
            //self.admin_client.restart(name);
        }

        pub fn check_for_update(&self) {
            println!("Check for update request");
        }

        pub fn update_request(&self) {
            println!("Update request");
        }

        pub fn get_stats(
            &self,
            vm: String,
        ) -> impl std::future::Future<Output = Result<StatsResponse, String>> {
            self.client_cmd_async(adminclient!(|client| client.get_stats(vm)))
        }

        pub fn set_locale(
            &self,
            locale: String,
        ) -> impl std::future::Future<Output = Result<(), String>> {
            self.client_cmd_async(adminclient!(|client| client.set_locale(locale)))
        }

        pub fn set_timezone(
            &self,
            timezone: String,
        ) -> impl std::future::Future<Output = Result<(), String>> {
            self.client_cmd_async(adminclient!(|client| client.set_timezone(timezone)))
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

        fn get_timezone_display(tz: &str) -> String {
            tz.chars().map(|c| if c == '_' { ' ' } else { c }).collect()
        }

        fn get_timezones() -> Result<Vec<LanguageRegionEntry>, Box<dyn std::error::Error>> {
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
