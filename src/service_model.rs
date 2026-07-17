pub use givc_client::client::StatsResponse;
pub use givc_common::pb::admin::StartResponse;

use givc_client::endpoint::TlsConfig;
use glib::subclass::prelude::*;
use gtk::{self, gio, glib};

use crate::prelude::*;
use crate::service_gobject::ServiceGObject;
use crate::settings_action::UpdateServerAuthMode;
pub use crate::update_state::{UpdateActivity, UpdateState};

#[derive(Debug, Clone)]
pub struct HostSysinfoStatus {
    pub ghaf_version: String,
    pub secure_boot: Option<bool>,
    pub disk_encryption: Option<bool>,
}

mod imp {
    #![cfg_attr(feature = "mock", allow(unused_imports, dead_code))]

    use std::cell::{Cell, RefCell};
    use std::thread;
    use std::time::Duration;

    use anyhow::Context;
    use async_channel::Sender;
    use gio::{ListModel, subclass::prelude::*};
    #[cfg(not(feature = "mock"))]
    use givc_client::RegistryAuth;
    use givc_client::endpoint::TlsConfig;
    use givc_client::{self, AdminClient};
    #[cfg(not(feature = "mock"))]
    use givc_common::pb::admin::{AvailableUpdate, RegistryPullResult};
    use givc_common::{address::EndpointAddress, query::Event};
    use glib::JoinHandle;
    use glib::{Object, Properties, SourceId};
    use gtk::{gio, glib, prelude::*};
    use secrecy::ExposeSecret;
    use tokio::runtime::Builder;

    use super::{HostSysinfoStatus, StartResponse, StatsResponse, UpdateActivity, UpdateState};

    use crate::prelude::*;
    use crate::service_gobject::ServiceGObject;
    use crate::settings_action::UpdateServerAuthMode;

    type TaskSender = Sender<(Task, Sender<Response>)>;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ServiceModel)]
    pub struct ServiceModel {
        services: RefCell<Vec<ServiceGObject>>,
        update_state: UpdateState,
        #[cfg(feature = "mock")]
        mock_check_attempts: Cell<u32>,
        #[cfg(not(feature = "mock"))]
        selected_update_reference: RefCell<Option<String>>,
        #[cfg(not(feature = "mock"))]
        downloaded_manifest_path: RefCell<Option<String>>,

        #[property(set = ServiceModel::set_address, get)]
        address: RefCell<String>,

        #[property(set = ServiceModel::set_port, get = ServiceModel::get_port, type = u32)]
        port: Cell<u16>,

        reconnect_timeout: RefCell<Option<SourceId>>,
        tls_info: RefCell<Option<(String, TlsConfig)>>,
        task_runner: RefCell<Option<TaskSender>>,
        #[cfg(not(feature = "mock"))]
        join_handle: RefCell<Option<JoinHandle<()>>>,
    }

    impl ListModelImpl for ServiceModel {
        fn item_type(&self) -> glib::types::Type {
            ServiceGObject::static_type()
        }

        #[allow(clippy::cast_possible_truncation)]
        fn n_items(&self) -> u32 {
            self.services.borrow().len() as u32
        }

        fn item(&self, idx: u32) -> Option<Object> {
            self.services
                .borrow()
                .get(idx as usize)
                .map(|dev| dev.clone().upcast())
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServiceModel {
        const NAME: &'static str = "ServiceModel";
        type Type = super::ServiceModel;
        type Interfaces = (ListModel,);
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServiceModel {}

    #[derive(Debug)]
    pub enum Response {
        Empty,
        Stats(StatsResponse),
        Start(StartResponse),
        SysinfoStatus(HostSysinfoStatus),
        #[cfg(not(feature = "mock"))]
        RegistryUpdates(Vec<AvailableUpdate>),
        #[cfg(not(feature = "mock"))]
        RegistryChangelog(String),
        #[cfg(not(feature = "mock"))]
        RegistryPullResult(RegistryPullResult),
        Error(anyhow::Error),
    }

    impl From<Result<(), anyhow::Error>> for Response {
        fn from(r: Result<(), anyhow::Error>) -> Response {
            r.map_or_else(Response::Error, |()| Response::Empty)
        }
    }

    impl std::convert::TryFrom<Response> for () {
        type Error = anyhow::Error;

        fn try_from(r: Response) -> Result<(), Self::Error> {
            match r {
                Response::Empty => Ok(()),
                Response::Error(e) => Err(e),
                _ => anyhow::bail!("Unexpected response"),
            }
        }
    }

    impl From<Result<StartResponse, anyhow::Error>> for Response {
        fn from(r: Result<StartResponse, anyhow::Error>) -> Response {
            r.map_or_else(Response::Error, Response::Start)
        }
    }

    impl std::convert::TryFrom<Response> for StartResponse {
        type Error = anyhow::Error;

        fn try_from(r: Response) -> Result<StartResponse, Self::Error> {
            match r {
                Response::Start(s) => Ok(s),
                Response::Error(e) => Err(e),
                _ => anyhow::bail!("Unexpected response"),
            }
        }
    }

    impl From<Result<StatsResponse, anyhow::Error>> for Response {
        fn from(r: Result<StatsResponse, anyhow::Error>) -> Response {
            r.map_or_else(Response::Error, Response::Stats)
        }
    }

    impl std::convert::TryFrom<Response> for StatsResponse {
        type Error = anyhow::Error;

        fn try_from(r: Response) -> Result<StatsResponse, Self::Error> {
            match r {
                Response::Stats(s) => Ok(s),
                Response::Error(e) => Err(e),
                _ => anyhow::bail!("Unexpected response"),
            }
        }
    }

    impl From<Result<HostSysinfoStatus, anyhow::Error>> for Response {
        fn from(r: Result<HostSysinfoStatus, anyhow::Error>) -> Response {
            r.map_or_else(Response::Error, Response::SysinfoStatus)
        }
    }

    impl std::convert::TryFrom<Response> for HostSysinfoStatus {
        type Error = anyhow::Error;

        fn try_from(r: Response) -> Result<HostSysinfoStatus, Self::Error> {
            match r {
                Response::SysinfoStatus(status) => Ok(status),
                Response::Error(e) => Err(e),
                _ => anyhow::bail!("Unexpected response"),
            }
        }
    }

    #[cfg(not(feature = "mock"))]
    impl From<Result<Vec<AvailableUpdate>, anyhow::Error>> for Response {
        fn from(r: Result<Vec<AvailableUpdate>, anyhow::Error>) -> Response {
            r.map_or_else(Response::Error, Response::RegistryUpdates)
        }
    }

    #[cfg(not(feature = "mock"))]
    impl std::convert::TryFrom<Response> for Vec<AvailableUpdate> {
        type Error = anyhow::Error;

        fn try_from(r: Response) -> Result<Vec<AvailableUpdate>, Self::Error> {
            match r {
                Response::RegistryUpdates(updates) => Ok(updates),
                Response::Error(e) => Err(e),
                _ => anyhow::bail!("Unexpected response"),
            }
        }
    }

    #[cfg(not(feature = "mock"))]
    impl From<Result<String, anyhow::Error>> for Response {
        fn from(r: Result<String, anyhow::Error>) -> Response {
            r.map_or_else(Response::Error, Response::RegistryChangelog)
        }
    }

    #[cfg(not(feature = "mock"))]
    impl std::convert::TryFrom<Response> for String {
        type Error = anyhow::Error;

        fn try_from(r: Response) -> Result<String, Self::Error> {
            match r {
                Response::RegistryChangelog(changelog) => Ok(changelog),
                Response::Error(e) => Err(e),
                _ => anyhow::bail!("Unexpected response"),
            }
        }
    }

    #[cfg(not(feature = "mock"))]
    impl From<Result<RegistryPullResult, anyhow::Error>> for Response {
        fn from(r: Result<RegistryPullResult, anyhow::Error>) -> Response {
            r.map_or_else(Response::Error, Response::RegistryPullResult)
        }
    }

    #[cfg(not(feature = "mock"))]
    impl std::convert::TryFrom<Response> for RegistryPullResult {
        type Error = anyhow::Error;

        fn try_from(r: Response) -> Result<RegistryPullResult, Self::Error> {
            match r {
                Response::RegistryPullResult(result) => Ok(result),
                Response::Error(e) => Err(e),
                _ => anyhow::bail!("Unexpected response"),
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

    impl ServiceModel {
        #[cfg(not(feature = "mock"))]
        pub(super) async fn get_sysinfo_status_from_host(
            &self,
        ) -> Result<HostSysinfoStatus, anyhow::Error> {
            debug!("ServiceModel: querying host sysinfo status via admin RPC");
            self.client_cmd(async move |client| {
                let status = client.sysinfo().await?;
                Ok(HostSysinfoStatus {
                    ghaf_version: status.ghaf_version,
                    secure_boot: status.secure_boot,
                    disk_encryption: status.disk_encrypted,
                })
            })
            .await
        }

        pub fn delayed_reconnect(&self) {
            let delay = std::time::Duration::from_millis(100);
            let mut guard = self.reconnect_timeout.borrow_mut();
            let model = self.obj().clone();

            if let Some(source_id) = guard.replace(glib::timeout_add_local_once(delay, move || {
                glib::spawn_future_local(async move {
                    model.imp().reconnect().await;
                });
            })) {
                source_id.remove();
            }
        }

        #[allow(dead_code)]
        pub(super) fn client_cmd_cb<T, R>(
            &self,
            task: T,
            cb: impl Fn(Result<R, anyhow::Error>) + 'static,
        ) where
            T: AsyncFnOnce(&AdminClient) -> Result<R, anyhow::Error> + Send + Sync + 'static,
            Result<R, anyhow::Error>: Into<Response>,
            R: std::convert::TryFrom<Response, Error = anyhow::Error> + 'static,
        {
            glib::spawn_future_local(glib::clone!(
                #[strong(rename_to = model)]
                self.obj(),
                async move {
                    let task = model.imp().client_cmd(task);
                    cb(task.await)
                }
            ));
        }

        pub(super) async fn client_cmd<T, R>(&self, task: T) -> Result<R, anyhow::Error>
        where
            T: AsyncFnOnce(&AdminClient) -> Result<R, anyhow::Error> + Send + Sync + 'static,
            Result<R, anyhow::Error>: Into<Response>,
            R: std::convert::TryFrom<Response, Error = anyhow::Error>,
        {
            let (res_tx, res_rx) = async_channel::bounded(3);
            let tr = self
                .task_runner
                .borrow()
                .as_ref()
                .cloned()
                .context("Not connected")?;

            tr.send((
                Box::new(|client: &AdminClient| {
                    let task = task(client);
                    Box::pin(async move { task.await.into() })
                }),
                res_tx,
            ))
            .await?;
            res_rx.recv().await?.try_into()
        }

        fn set_address(&self, address: String) {
            *self.address.borrow_mut() = address;
            self.delayed_reconnect();
        }

        fn set_port(&self, port: u32) {
            self.port.set(port.try_into().unwrap_or(0));
            self.delayed_reconnect();
        }

        fn get_port(&self) -> u32 {
            u32::from(self.port.get())
        }

        #[allow(clippy::cast_possible_truncation)]
        fn extend<T>(&self, iter: impl IntoIterator<Item = T>)
        where
            ServiceGObject: From<T>,
        {
            use givc_common::query::{TrustLevel, VMStatus};
            use givc_common::types::{ServiceType, VmType};

            let n = self.services.borrow().len();
            if n == 0 {
                self.services.borrow_mut().extend(
                    iter.into_iter()
                        .map(ServiceGObject::from)
                        .chain(Some(ServiceGObject::new(
                            "ghaf-host",
                            "Host operating system",
                            VMStatus::Running,
                            TrustLevel::Warning,
                            ServiceType::VM,
                            Some("ghaf-host"),
                            VmType::Host,
                        ))),
                );
                self.services
                    .borrow_mut()
                    .sort_by_cached_key(ServiceGObject::sort_key);
                let n = self.services.borrow().len();
                self.obj().items_changed(0, 0, n as u32);
            } else {
                for service in iter.into_iter().map(ServiceGObject::from) {
                    let Err(pos) = self
                        .services
                        .borrow()
                        .binary_search_by_key(&service.sort_key(), ServiceGObject::sort_key)
                    else {
                        continue;
                    };
                    self.services.borrow_mut().insert(pos, service);
                    self.obj().items_changed(pos as u32, 0, 1);
                }
            }
        }

        #[cfg(feature = "mock")]
        fn fill_by_mock_data(&self) {
            use givc_common::query::{TrustLevel, VMStatus};
            use givc_common::types::{ServiceType, VmType};

            self.extend([
                ServiceGObject::new(
                    "microvm@zathura-vm.service",
                    "This is the file.pdf and very very long description",
                    VMStatus::Running,
                    TrustLevel::NotSecure,
                    ServiceType::VM,
                    Some("zathura-vm"),
                    VmType::AppVM,
                ),
                ServiceGObject::new(
                    "zathura@1.service",
                    "Zathura",
                    VMStatus::Paused,
                    TrustLevel::Secure,
                    ServiceType::App,
                    Some("zathura-vm"),
                    VmType::AppVM,
                ),
                ServiceGObject::new(
                    "chrome@1.service",
                    "Google Chrome",
                    VMStatus::Paused,
                    TrustLevel::Secure,
                    ServiceType::App,
                    Some("TestVM"),
                    VmType::AppVM,
                ),
                ServiceGObject::new(
                    "appflowy@1.service",
                    "AppFlowy",
                    VMStatus::Running,
                    TrustLevel::Secure,
                    ServiceType::Svc,
                    Some("appflowy-vm"),
                    VmType::AppVM,
                ),
                ServiceGObject::new(
                    "microvm@admin-vm.service",
                    "AdminVM",
                    VMStatus::Running,
                    TrustLevel::Secure,
                    ServiceType::VM,
                    Some("admin-vm"),
                    VmType::AdmVM,
                ),
            ]);
        }

        pub(super) fn find(
            &self,
            pred: impl Fn(&ServiceGObject) -> bool,
        ) -> Option<(usize, ServiceGObject)> {
            self.services
                .borrow()
                .iter()
                .enumerate()
                .find_map(|(pos, obj)| pred(obj).then(|| (pos, obj.clone())))
        }

        pub(super) fn set_tls_info(&self, name: String, config: TlsConfig) {
            *self.tls_info.borrow_mut() = Some((name, config));
            self.delayed_reconnect();
        }

        #[cfg(not(feature = "mock"))]
        fn set_selected_update_reference(&self, reference: Option<String>) {
            *self.selected_update_reference.borrow_mut() = reference;
        }

        #[cfg(not(feature = "mock"))]
        fn set_downloaded_manifest_path(&self, manifest_path: Option<String>) {
            *self.downloaded_manifest_path.borrow_mut() = manifest_path;
        }

        #[cfg(not(feature = "mock"))]
        fn selected_update_reference(&self) -> Option<String> {
            self.selected_update_reference.borrow().clone()
        }

        #[cfg(not(feature = "mock"))]
        fn registry_reference(reference: &str) -> anyhow::Result<String> {
            let reference = reference.trim();
            if reference.is_empty() {
                anyhow::bail!("update server reference is not configured");
            }

            Ok(reference.to_string())
        }

        #[cfg(not(feature = "mock"))]
        fn registry_auth(auth_mode: UpdateServerAuthMode) -> anyhow::Result<Option<RegistryAuth>> {
            match auth_mode {
                UpdateServerAuthMode::Anonymous => Ok(None),
                UpdateServerAuthMode::UserPass { username, password } => {
                    if username.is_empty() || password.expose_secret().is_empty() {
                        anyhow::bail!(
                            "update server user/password auth requires both username and password"
                        );
                    }
                    Ok(Some(RegistryAuth::Basic {
                        username,
                        password: password.expose_secret().to_owned(),
                    }))
                }
                UpdateServerAuthMode::OAuth { .. } => {
                    anyhow::bail!("OAuth update server auth is not implemented yet")
                }
            }
        }

        pub(super) fn update_state(&self) -> UpdateState {
            self.update_state.clone()
        }

        #[cfg_attr(not(feature = "mock"), allow(dead_code))]
        pub(super) fn update_state_with_notifications_frozen<F>(&self, f: F)
        where
            F: FnOnce(&UpdateState),
        {
            let _freeze = self.update_state.freeze_notify();
            f(&self.update_state);
        }

        #[cfg_attr(not(feature = "mock"), allow(dead_code))]
        async fn load_current_version(&self) -> Result<String, anyhow::Error> {
            Ok(self
                .obj()
                .get_sysinfo_status_from_host()
                .await?
                .ghaf_version)
        }

        #[cfg(feature = "mock")]
        fn seed_mock_update_state(&self, current_version: String) {
            self.update_state_with_notifications_frozen(|state| {
                state.set_current_version(current_version);
                state.set_available_version("0.0.1-mock");
                state.set_changelog(
                    "Mock update:\n\
                     - Re-frobnicate frobnicators\n\
                     - De-spaghettify the update pipeline\n\
                     - Encourage the progress bar to keep its day job\n\
                     - Reduce existential dread in the installer",
                );
                state.set_download_size_bytes(15 * 1024 * 1024 * 1024);
                state.set_activity(UpdateActivity::Checked);
            });
        }

        #[cfg(feature = "mock")]
        fn seed_mock_no_update_state(&self, current_version: String) {
            self.update_state_with_notifications_frozen(|state| {
                state.set_current_version(current_version);
                state.set_available_version(String::new());
                state.set_changelog(String::new());
                state.set_download_size_bytes(0);
                state.set_activity(UpdateActivity::Checked);
            });
        }

        #[cfg(feature = "mock")]
        #[allow(clippy::unused_async)]
        async fn reconnect(&self) {
            use givc_common::query::{TrustLevel, VMStatus};
            use givc_common::types::{ServiceType, VmType};
            self.fill_by_mock_data();

            glib::spawn_future_local(glib::clone!(
                #[strong(rename_to = model)]
                self.obj(),
                async move {
                    glib::timeout_future_seconds(3).await;
                    model.imp().extend(Some(ServiceGObject::new(
                        "microvm@appflowy-vm.service",
                        "AppFlow VM",
                        VMStatus::Running,
                        TrustLevel::NotSecure,
                        ServiceType::VM,
                        Some("appflowy-vm"),
                        VmType::AppVM,
                    )));
                    glib::timeout_future_seconds(3).await;
                    model.imp().extend(Some(ServiceGObject::new(
                        "zathura@2.service",
                        "Zathura",
                        VMStatus::Paused,
                        TrustLevel::Secure,
                        ServiceType::App,
                        Some("zathura-vm"),
                        VmType::AppVM,
                    )));
                    glib::timeout_future_seconds(3).await;
                    model.imp().extend(Some(ServiceGObject::new(
                        "givc-appflowy-vm.service",
                        "Zathura agent",
                        VMStatus::Running,
                        TrustLevel::Secure,
                        ServiceType::Mgr,
                        None,
                        VmType::AppVM,
                    )));
                }
            ));
        }

        #[cfg(feature = "mock")]
        pub(super) async fn check_for_update(
            &self,
            _reference: String,
            _auth_mode: UpdateServerAuthMode,
            _insecure: bool,
        ) -> Result<(), anyhow::Error> {
            let current_version = self.load_current_version().await.unwrap_or_else(|err| {
                warn!("ServiceModel: failed to load current version: {err}");
                "unknown".to_string()
            });
            let attempt = self.mock_check_attempts.get();
            self.mock_check_attempts.set(attempt.saturating_add(1));

            self.update_state_with_notifications_frozen(|state| {
                state.set_current_version(current_version.clone());
                state.set_available_version(String::new());
                state.set_changelog(String::new());
                state.set_download_size_bytes(0);
                state.set_activity(UpdateActivity::Checking);
            });

            for _ in 1..=4 {
                glib::timeout_future(Duration::from_millis(250)).await;
                self.update_state_with_notifications_frozen(|state| {
                    state.set_current_version(current_version.clone());
                    state.set_available_version(String::new());
                    state.set_changelog(String::new());
                    state.set_download_size_bytes(0);
                    state.set_activity(UpdateActivity::Checking);
                });
            }

            if attempt == 0 {
                self.seed_mock_no_update_state(current_version);
            } else {
                self.seed_mock_update_state(current_version);
            }
            Ok(())
        }

        #[cfg(feature = "mock")]
        pub(super) async fn download_update(
            &self,
            _reference: String,
            _auth_mode: UpdateServerAuthMode,
            _insecure: bool,
        ) -> Result<(), anyhow::Error> {
            if self.update_state.available_version().is_empty() {
                let current_version = self.load_current_version().await.unwrap_or_else(|err| {
                    warn!("ServiceModel: failed to load current version before download: {err}");
                    "unknown".to_string()
                });
                self.seed_mock_update_state(current_version);
            }

            self.update_state_with_notifications_frozen(|state| {
                state.set_activity(UpdateActivity::Downloading { progress: 0.0 });
            });

            for step in 1..=10 {
                glib::timeout_future_seconds(1).await;
                self.update_state_with_notifications_frozen(|state| {
                    state.set_activity(UpdateActivity::Downloading {
                        progress: f64::from(step) / 10.0,
                    });
                });
            }

            self.update_state_with_notifications_frozen(|state| {
                state.set_activity(UpdateActivity::Downloaded);
            });
            Ok(())
        }

        #[cfg(feature = "mock")]
        pub(super) async fn apply_update(&self) -> Result<(), anyhow::Error> {
            if !matches!(self.update_state.activity(), UpdateActivity::Downloaded) {
                anyhow::bail!("update package is not ready");
            }

            self.update_state_with_notifications_frozen(|state| {
                state.set_activity(UpdateActivity::Installing { progress: 0.0 });
            });

            for step in 1..=10 {
                glib::timeout_future_seconds(1).await;
                self.update_state_with_notifications_frozen(|state| {
                    state.set_activity(UpdateActivity::Installing {
                        progress: f64::from(step) / 10.0,
                    });
                });
            }

            let installed_version = self.update_state.available_version();
            if installed_version.is_empty() {
                anyhow::bail!("update package is not ready");
            }
            self.update_state_with_notifications_frozen(|state| {
                state.set_current_version(installed_version.clone());
                state.set_available_version(String::new());
                state.set_changelog(String::new());
                state.set_download_size_bytes(0);
                state.set_activity(UpdateActivity::Installed);
            });
            Ok(())
        }

        #[cfg(feature = "mock")]
        #[allow(clippy::unused_async)]
        pub(super) async fn start_update_server_oauth_flow(&self) -> Result<(), anyhow::Error> {
            info!("ServiceModel: mocked update-server OAuth flow requested");
            Ok(())
        }

        #[cfg(not(feature = "mock"))]
        #[allow(clippy::cast_possible_truncation)]
        async fn reconnect(&self) {
            let _ = self.task_runner.borrow_mut().take();
            let join = self.join_handle.borrow_mut().take();
            if let Some(join) = join {
                let _ = join.await;
            }
            if self.address.borrow().is_empty() || self.port.get() == 0 {
                return;
            }
            let address = EndpointAddress::Tcp {
                addr: self.address.borrow().clone(),
                port: self.port.get(),
            };
            let tls_info = self.tls_info.borrow().as_ref().cloned();

            let (event_tx, event_rx) = async_channel::unbounded();
            let (task_tx, task_rx) =
                async_channel::bounded::<(Task, async_channel::Sender<Response>)>(1);

            *self.task_runner.borrow_mut() = Some(task_tx);
            thread::spawn(move || {
                Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async move {
                        let timeout_duration = Duration::from_secs(5);
                        let admin_client = AdminClient::from_endpoint_address(address, tls_info);
                        let result = {
                            tokio::select! {
                                () = tokio::time::sleep(timeout_duration) => {
                                    warn!("Watch call timeout");
                                    return;
                                },
                                result = admin_client.watch() => match result {
                                    Ok(result) => result,
                                    Err(e) => {
                                        error!("Watch call failed: {e}");
                                        return;
                                    }
                                },
                                () = async {
                                    while task_rx.recv().await.is_ok() {
                                        debug!("Not yet connected, task ignored");
                                    }
                                } => return,
                            }
                        };
                        debug!("Connected!");

                        let _ = event_tx.send((result.channel, result.initial)).await;
                        while let Ok((task, resp)) = task_rx.recv().await {
                            let res = task(&admin_client).await;
                            let _ = resp.send(res).await;
                        }
                    });
            });

            *self.join_handle.borrow_mut() = Some(glib::spawn_future_local(glib::clone!(
                #[strong(rename_to = model)]
                self.obj(),
                async move {
                    if let Ok((channel, initial)) = event_rx.recv().await {
                        let this = model.imp();
                        let n = this.services.borrow().len();
                        if n > 0 {
                            this.services.borrow_mut().clear();
                            model.items_changed(0, n as u32, 0);
                        }
                        this.extend(initial);

                        while let Ok(event) = channel.recv().await {
                            match event {
                                Event::UnitStatusChanged(result) => {
                                    debug!("Status: {result:?}");
                                    if let Some((_, obj)) =
                                        this.find(|obj| obj.name() == result.name)
                                    {
                                        obj.update(result);
                                    }
                                }
                                Event::UnitShutdown(result) => {
                                    debug!("Shutdown info: {result:?}");
                                    //Remove service/app, update VM
                                    #[allow(clippy::cast_possible_truncation)]
                                    if let Some((pos, obj)) =
                                        this.find(|obj| obj.name() == result.name)
                                    {
                                        if obj.is_vm() {
                                            obj.update(result);
                                        } else {
                                            this.services.borrow_mut().remove(pos);
                                            model.items_changed(pos as u32, 1, 0);
                                        }
                                    }
                                }
                                Event::UnitRegistered(result) => {
                                    debug!("Unit registered {result:?}");
                                    this.extend(Some(result));
                                }
                            }
                        }
                    }
                }
            )));
        }

        #[cfg(not(feature = "mock"))]
        pub(super) async fn check_for_update(
            &self,
            reference: String,
            auth_mode: UpdateServerAuthMode,
            insecure: bool,
        ) -> Result<(), anyhow::Error> {
            let current_version = self.load_current_version().await.unwrap_or_else(|err| {
                warn!("ServiceModel: failed to load current version: {err}");
                "unknown".to_string()
            });
            let reference = Self::registry_reference(&reference)?;
            let auth = Self::registry_auth(auth_mode)?;
            let auth_for_changelog = auth.clone();

            self.set_selected_update_reference(None);
            self.set_downloaded_manifest_path(None);
            self.update_state_with_notifications_frozen(|state| {
                state.set_current_version(current_version.clone());
                state.set_available_version(String::new());
                state.set_changelog(String::new());
                state.set_download_size_bytes(0);
                state.set_activity(UpdateActivity::Checking);
            });

            let updates = self
                .client_cmd(async move |client| {
                    client.discover_updates(reference, auth, insecure).await
                })
                .await?;

            if let Some(update) = updates.into_iter().next() {
                let selected_reference = format!("{}:{}", update.repository, update.tag);
                let changelog = {
                    let selected_reference = selected_reference.clone();
                    self.client_cmd(async move |client| {
                        client
                            .fetch_changelog(selected_reference, auth_for_changelog, insecure)
                            .await
                    })
                    .await
                    .unwrap_or_else(|err| {
                        warn!("ServiceModel: failed to fetch changelog: {err}");
                        String::new()
                    })
                };

                self.set_selected_update_reference(Some(selected_reference));
                self.set_downloaded_manifest_path(None);
                self.update_state_with_notifications_frozen(|state| {
                    state.set_available_version(update.version);
                    state.set_changelog(changelog);
                    state.set_download_size_bytes(0);
                    state.set_activity(UpdateActivity::Checked);
                });
            } else {
                self.update_state_with_notifications_frozen(|state| {
                    state.set_available_version(String::new());
                    state.set_changelog(String::new());
                    state.set_download_size_bytes(0);
                    state.set_activity(UpdateActivity::Checked);
                });
            }
            Ok(())
        }

        #[cfg(not(feature = "mock"))]
        #[allow(clippy::cast_precision_loss)]
        pub(super) async fn download_update(
            &self,
            reference: String,
            auth_mode: UpdateServerAuthMode,
            insecure: bool,
        ) -> Result<(), anyhow::Error> {
            if self.selected_update_reference().is_none() {
                self.check_for_update(reference.clone(), auth_mode.clone(), insecure)
                    .await?;
            }

            let Some(reference) = self.selected_update_reference() else {
                anyhow::bail!("no update is available to download");
            };
            let auth = Self::registry_auth(auth_mode)?;
            let destination = "/persist/sysupdate".to_string();
            let (progress_tx, progress_rx) = async_channel::unbounded::<f64>();
            let model = self.obj().clone();

            glib::spawn_future_local(async move {
                while let Ok(progress) = progress_rx.recv().await {
                    model.imp().update_state_with_notifications_frozen(|state| {
                        state.set_activity(UpdateActivity::Downloading {
                            progress: progress.clamp(0.0, 1.0),
                        });
                    });
                }
            });

            self.update_state_with_notifications_frozen(|state| {
                state.set_activity(UpdateActivity::Downloading { progress: 0.0 });
            });

            let result = self
                .client_cmd(async move |client| {
                    client
                        .pull_update(reference, destination, true, auth, insecure, move |progress| {
                            let progress_tx = progress_tx.clone();
                            async move {
                                let progress = match progress.event {
                                    Some(givc_common::pb::registry_pull_progress::Event::BlobDownloading(blob)) => {
                                        blob.total.map_or(0.0, |total| {
                                            if total == 0 {
                                                0.0
                                            } else {
                                                blob.downloaded as f64 / total as f64
                                            }
                                        })
                                    }
                                    Some(givc_common::pb::registry_pull_progress::Event::BlobVerified(_) |
givc_common::pb::registry_pull_progress::Event::ManifestWritten(_) |
givc_common::pb::registry_pull_progress::Event::Done(_)) => 1.0,
                                    Some(givc_common::pb::registry_pull_progress::Event::PullStarted(_)
                                        | givc_common::pb::registry_pull_progress::Event::Cancelled(_))
                                    | None => 0.0,
                                };
                                let _ = progress_tx.send(progress).await;
                            }
                        })
                        .await
                })
                .await;

            match result {
                Ok(result) => {
                    self.set_downloaded_manifest_path(Some(result.manifest_path));
                    self.update_state_with_notifications_frozen(|state| {
                        state.set_activity(UpdateActivity::Downloaded);
                    });
                    Ok(())
                }
                Err(err) => {
                    self.update_state_with_notifications_frozen(|state| {
                        state.set_activity(UpdateActivity::Checked);
                    });
                    Err(err)
                }
            }
        }

        #[cfg(not(feature = "mock"))]
        pub(super) async fn apply_update(&self) -> Result<(), anyhow::Error> {
            if !matches!(self.update_state.activity(), UpdateActivity::Downloaded) {
                anyhow::bail!("update package is not ready");
            }

            let Some(manifest_path) = self.downloaded_manifest_path.borrow().clone() else {
                anyhow::bail!("downloaded update manifest is missing");
            };
            let installed_version = self.update_state.available_version();
            if installed_version.is_empty() {
                anyhow::bail!("update package is not ready");
            }

            self.update_state_with_notifications_frozen(|state| {
                state.set_activity(UpdateActivity::Installing { progress: 0.0 });
            });

            let result = self
                .client_cmd(async move |client| client.image_install(manifest_path, true).await)
                .await;

            match result {
                Ok(()) => {
                    self.set_selected_update_reference(None);
                    self.set_downloaded_manifest_path(None);
                    self.update_state_with_notifications_frozen(|state| {
                        state.set_current_version(installed_version.clone());
                        state.set_available_version(String::new());
                        state.set_changelog(String::new());
                        state.set_download_size_bytes(0);
                        state.set_activity(UpdateActivity::Installed);
                    });
                    Ok(())
                }
                Err(err) => {
                    self.update_state_with_notifications_frozen(|state| {
                        state.set_activity(UpdateActivity::Downloaded);
                    });
                    Err(err)
                }
            }
        }

        #[cfg(not(feature = "mock"))]
        #[allow(clippy::unused_async)]
        pub(super) async fn start_update_server_oauth_flow(&self) -> Result<(), anyhow::Error> {
            unimplemented!("update server OAuth flow is mocked only for now")
        }
    }
}

glib::wrapper! {
    pub struct ServiceModel(ObjectSubclass<imp::ServiceModel>) @implements gio::ListModel;
}

impl Default for ServiceModel {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl ServiceModel {
    pub fn set_tls_info(&self, name: String, config: TlsConfig) {
        self.imp().set_tls_info(name, config);
    }

    pub fn current_update_state(&self) -> UpdateState {
        self.imp().update_state()
    }

    pub fn set_update_current_version(&self, current_version: String) {
        self.imp().update_state_with_notifications_frozen(|state| {
            state.set_current_version(current_version);
        });
    }

    pub async fn start_update_server_oauth_flow(&self) -> Result<(), anyhow::Error> {
        self.imp().start_update_server_oauth_flow().await
    }

    pub async fn start_service(&self, obj: ServiceGObject) -> Result<StartResponse, anyhow::Error> {
        let vm = obj.vm_name();

        if obj.is_vm() {
            self.imp()
                .client_cmd(async move |client| client.start_vm(vm).await)
                .await
        } else if obj.is_app() {
            let name = obj.display_name();
            self.imp()
                .client_cmd(async move |client| client.start_app(name, vm, vec![]).await)
                .await
        } else {
            let name = obj.name();
            self.imp()
                .client_cmd(async move |client| client.start_service(name, vm).await)
                .await
        }
    }

    pub async fn start_app_in_vm(
        &self,
        app: String,
        vm: String,
        args: Vec<String>,
    ) -> Result<StartResponse, anyhow::Error> {
        self.imp()
            .client_cmd(async move |client| client.start_app(app, vm, args).await)
            .await
    }

    #[allow(clippy::unused_async)]
    pub async fn restart_service(
        &self,
        _obj: &ServiceGObject,
    ) -> Result<StartResponse, anyhow::Error> {
        warn!("Restart is not implemented on client lib!");
        //no restart in admin_client
        //self.admin_client.restart(name);
        Err(anyhow::anyhow!("not implemented"))
    }

    pub async fn stop_service(&self, obj: &ServiceGObject) -> Result<(), anyhow::Error> {
        let name = obj.name();
        self.imp()
            .client_cmd(async move |client| client.stop(name).await)
            .await
    }

    pub async fn pause_service(&self, obj: &ServiceGObject) -> Result<(), anyhow::Error> {
        let name = obj.name();
        self.imp()
            .client_cmd(async move |client| client.pause(name).await)
            .await
    }

    pub async fn resume_service(&self, obj: &ServiceGObject) -> Result<(), anyhow::Error> {
        let name = obj.name();
        self.imp()
            .client_cmd(async move |client| client.resume(name).await)
            .await
    }

    pub async fn set_locale(&self, locale: String) -> Result<(), anyhow::Error> {
        self.imp()
            .client_cmd(async |client| client.set_locale(locale).await)
            .await
    }

    pub async fn set_timezone(&self, timezone: String) -> Result<(), anyhow::Error> {
        self.imp()
            .client_cmd(async |client| client.set_timezone(timezone).await)
            .await
    }

    #[cfg(not(feature = "mock"))]
    pub async fn get_stats(&self, vm: String) -> Result<StatsResponse, anyhow::Error> {
        self.imp()
            .client_cmd(async |client| client.get_stats(vm).await)
            .await
    }

    #[cfg(not(feature = "mock"))]
    pub async fn get_sysinfo_status_from_host(&self) -> Result<HostSysinfoStatus, anyhow::Error> {
        self.imp().get_sysinfo_status_from_host().await
    }

    #[cfg(feature = "mock")]
    #[allow(clippy::unused_self)]
    pub fn get_stats(
        &self,
        _vm: String,
    ) -> impl std::future::Future<Output = Result<StatsResponse, anyhow::Error>> {
        use givc_common::pb::stats::{MemoryStats, ProcessStats};
        async {
            Ok(StatsResponse {
                memory: Some(MemoryStats {
                    total: 200_000_000,
                    available: 100_000_000,
                    free: 50_000_000,
                    ..Default::default()
                }),
                process: Some(ProcessStats {
                    user_cycles: 100_000,
                    sys_cycles: 50_000,
                    total_cycles: 200_000,
                    ..Default::default()
                }),
                ..Default::default()
            })
        }
    }

    #[cfg(feature = "mock")]
    #[allow(clippy::unused_async, clippy::unused_self)]
    pub async fn get_sysinfo_status_from_host(&self) -> Result<HostSysinfoStatus, anyhow::Error> {
        Ok(HostSysinfoStatus {
            ghaf_version: "0.0.0-mock".to_string(),
            secure_boot: Some(false),
            disk_encryption: Some(false),
        })
    }

    pub async fn check_for_update(
        &self,
        reference: String,
        auth_mode: UpdateServerAuthMode,
        insecure: bool,
    ) -> Result<(), anyhow::Error> {
        self.imp()
            .check_for_update(reference, auth_mode, insecure)
            .await
    }

    pub async fn download_update(
        &self,
        reference: String,
        auth_mode: UpdateServerAuthMode,
        insecure: bool,
    ) -> Result<(), anyhow::Error> {
        self.imp()
            .download_update(reference, auth_mode, insecure)
            .await
    }

    pub async fn update_request(&self) -> Result<(), anyhow::Error> {
        self.imp().apply_update().await
    }
}
