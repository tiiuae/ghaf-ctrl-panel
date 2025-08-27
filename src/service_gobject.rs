use gtk::glib::{self, Object};
use gtk::prelude::*;

use givc_common::query::{QueryResult, TrustLevel, VMStatus};
use givc_common::types::{ServiceType, VmType};

use crate::prelude::*;
use crate::wireguard_vms::static_contains;

mod imp {
    use gtk::glib::{self, Properties};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    use givc_common::query::{TrustLevel, VMStatus};
    use givc_common::types::VmType;

    pub struct ServiceData {
        pub name: String,         //unique service name, used as id
        pub display_name: String, //user-friendly name
        pub is_vm: bool,
        pub is_app: bool,
        pub vm_name: String, //for apps running in VMs
        pub vm_type: VmType,
        pub details: String,
        pub status: VMStatus,
        pub trust_level: TrustLevel,
        pub has_wireguard: bool,
    }

    impl Default for ServiceData {
        fn default() -> Self {
            Self {
                name: String::new(),
                display_name: String::new(),
                is_vm: false,
                is_app: false,
                vm_name: String::new(),
                vm_type: VmType::Host,
                details: String::new(),
                status: VMStatus::default(),
                trust_level: TrustLevel::default(),
                has_wireguard: false,
            }
        }
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ServiceGObject)]
    pub struct ServiceGObject {
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "display-name", get, set, type = String, member = display_name)]
        #[property(name = "is-vm", get, set, type = bool, member = is_vm)]
        #[property(name = "is-app", get, set, type = bool, member = is_app)]
        #[property(name = "vm-name", get, set, type = String, member = vm_name)]
        #[property(name = "vm-type", get, set, type = VmType, member = vm_type, builder(VmType::Host))]
        #[property(name = "details", get, set, type = String, member = details)]
        #[property(name = "status", get, set, type = VMStatus, member = status, builder(VMStatus::default()))]
        #[property(name = "trust-level", get, set, type = TrustLevel, member = trust_level, builder(TrustLevel::default()))]
        #[property(name = "has-wireguard", get, set, type = bool, member = has_wireguard)]
        pub data: RefCell<ServiceData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServiceGObject {
        const NAME: &'static str = "VMObject";
        type Type = super::ServiceGObject;
        type ParentType = glib::Object;
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for ServiceGObject {}
}

glib::wrapper! {
    pub struct ServiceGObject(ObjectSubclass<imp::ServiceGObject>);
}

impl ServiceGObject {
    pub fn new(
        name: &str,
        details: &str,
        status: impl Into<VMStatus>,
        trust_level: impl Into<TrustLevel>,
        service_type: ServiceType,
        vm_name: Option<&str>,
        vm_type: VmType,
    ) -> Self {
        let is_vm = service_type == ServiceType::VM;
        let is_app = service_type == ServiceType::App;
        let status = status.into();

        let display_name = if is_vm {
            vm_name.unwrap_or("")
        } else if is_app {
            name.strip_suffix(".service")
                .and_then(|name| name.rsplit_once('@'))
                .and_then(|(name, number)| {
                    number
                        .chars()
                        .by_ref()
                        .all(|c| c.is_ascii_digit())
                        .then_some(name)
                })
                .unwrap_or("")
        } else {
            ""
        };

        debug!(
            "ServiceGObject::new: name: {name}, display_name: {display_name}, \
                is_vm: {is_vm}, is_app: {is_app}, vm_name: {vm_name:?}, \
                details: {details}, status: {status}",
        );

        let has_wireguard = is_vm && vm_name.is_some_and(static_contains);

        Object::builder()
            .property("name", name)
            .property("display-name", display_name)
            .property("is-vm", is_vm)
            .property("is-app", is_app)
            .property("vm-name", vm_name.unwrap_or_default())
            .property("vm-type", vm_type)
            .property("details", details)
            //for demo
            .property("status", status)
            .property("trust-level", trust_level.into()) //trust_level as u8)
            .property("has-wireguard", has_wireguard)
            .build()
    }

    pub fn update(&self, query_result: QueryResult) {
        self.set_property("details", query_result.description);
        self.set_property("status", query_result.status);
    }

    pub fn is_vm_running(&self) -> bool {
        self.is_vm() && matches!(self.status(), VMStatus::Running)
    }

    pub fn is_service(&self) -> bool {
        !self.is_vm() && !self.is_app()
    }

    pub fn sort_key(&self) -> (bool, String, bool, String) {
        let vm_name = self.vm_name();
        (
            &vm_name != "ghaf-host",
            self.vm_name(),
            !self.is_vm(),
            self.name(),
        )
    }
}

impl From<QueryResult> for ServiceGObject {
    fn from(
        QueryResult {
            name,
            description,
            status,
            trust_level,
            service_type,
            vm_name,
            vm_type,
            ..
        }: QueryResult,
    ) -> Self {
        Self::new(
            &name,
            &description,
            status,
            trust_level,
            service_type,
            vm_name.as_deref(),
            vm_type,
        )
    }
}
