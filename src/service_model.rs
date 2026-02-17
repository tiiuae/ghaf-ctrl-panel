pub use givc_client::client::StatsResponse;
pub use givc_common::pb::admin::StartResponse;

use givc_client::endpoint::TlsConfig;
use glib::subclass::prelude::*;
use gtk::{self, gio, glib};

use crate::prelude::*;
use crate::service_gobject::ServiceGObject;

#[derive(Debug, Clone)]
pub struct HostSysinfoStatus {
    pub ghaf_version: String,
    pub secure_boot: String,
    pub disk_encryption: String,
}

mod imp {
    #![cfg_attr(feature = "mock", allow(unused_imports, dead_code))]

    use std::cell::{Cell, RefCell};
    use std::thread;
    use std::time::Duration;

    use anyhow::Context;
    use async_channel::Sender;
    use gio::{ListModel, subclass::prelude::*};
    use givc_client::endpoint::TlsConfig;
    use givc_client::{self, AdminClient};
    use givc_common::{address::EndpointAddress, query::Event};
    use glib::JoinHandle;
    use glib::{Object, Properties, SourceId};
    use gtk::{gio, glib, prelude::*};
    use tokio::runtime::Builder;

    use super::{HostSysinfoStatus, StartResponse, StatsResponse};

    use crate::prelude::*;
    use crate::service_gobject::ServiceGObject;

    type TaskSender = Sender<(Task, Sender<Response>)>;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ServiceModel)]
    pub struct ServiceModel {
        services: RefCell<Vec<ServiceGObject>>,

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

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for ServiceModel {}

    #[derive(Debug)]
    pub enum Response {
        Empty,
        Stats(StatsResponse),
        Start(StartResponse),
        SysinfoStatus(HostSysinfoStatus),
        Error(anyhow::Error),
    }

    impl From<Result<(), anyhow::Error>> for Response {
        fn from(r: Result<(), anyhow::Error>) -> Response {
            match r {
                Ok(()) => Response::Empty,
                Err(e) => Response::Error(e),
            }
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
            match r {
                Ok(r) => Response::Start(r),
                Err(e) => Response::Error(e),
            }
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
            match r {
                Ok(r) => Response::Stats(r),
                Err(e) => Response::Error(e),
            }
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
            match r {
                Ok(r) => Response::SysinfoStatus(r),
                Err(e) => Response::Error(e),
            }
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
            let sysinfo = self
                .client_cmd(async move |client| {
                    let status = client.sysinfo().await?;
                    Ok(HostSysinfoStatus {
                        ghaf_version: status.ghaf_version,
                        secure_boot: status.secure_boot,
                        disk_encryption: status.disk_encrypted,
                    })
                })
                .await?;
            Ok(sysinfo)
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
    #[allow(clippy::unused_self)]
    pub async fn get_sysinfo_status_from_host(&self) -> Result<HostSysinfoStatus, anyhow::Error> {
        Ok(HostSysinfoStatus {
            ghaf_version: "0.0.0-mock".to_string(),
            secure_boot: "Disabled".to_string(),
            disk_encryption: "Disabled".to_string(),
        })
    }

    #[allow(clippy::unused_async, clippy::unused_self)]
    pub async fn check_for_update(&self) -> Result<(), anyhow::Error> {
        warn!("Check for update request");
        Ok(())
    }

    #[allow(clippy::unused_self)]
    pub fn update_request(&self) {
        warn!("Update request");
    }
}
