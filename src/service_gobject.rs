use gtk::glib::{self, Object};
use gtk::prelude::*;
use regex::Regex;

use givc_common::query::QueryResult;
use givc_common::types::ServiceType;

use crate::prelude::*;
use crate::trust_level::TrustLevel;
use crate::vm_status::VMStatus;
use crate::wireguard_vms::static_contains;

mod imp {
    use gtk::glib::{self, Properties};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    use crate::trust_level::TrustLevel;
    use crate::vm_status::VMStatus;

    #[derive(Default)]
    pub struct ServiceData {
        pub name: String,         //unique service name, used as id
        pub display_name: String, //user-friendly name
        pub is_vm: bool,
        pub is_app: bool,
        pub vm_name: String, //for apps running in VMs
        pub details: String,
        pub status: VMStatus,
        pub trust_level: TrustLevel,
        pub has_wireguard: bool,
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ServiceGObject)]
    pub struct ServiceGObject {
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "display-name", get, set, type = String, member = display_name)]
        #[property(name = "is-vm", get, set, type = bool, member = is_vm)]
        #[property(name = "is-app", get, set, type = bool, member = is_app)]
        #[property(name = "vm-name", get, set, type = String, member = vm_name)]
        #[property(name = "details", get, set, type = String, member = details)]
        #[property(name = "status", get, set, type = VMStatus, member = status, builder(VMStatus::Running))]
        #[property(name = "trust-level", get, set, type = TrustLevel, member = trust_level, builder(TrustLevel::Secure))]
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
        name: String,
        details: String,
        status: impl Into<VMStatus>,
        trust_level: impl Into<TrustLevel>,
        service_type: ServiceType,
        vm_name: Option<String>,
    ) -> Self {
        let is_vm = service_type == ServiceType::VM;
        let is_app = service_type == ServiceType::App;
        let status = status.into();

        let display_name = if is_vm {
            //vm_name
            let re = Regex::new(r"^microvm@([^@-]+)-.+$").unwrap();
            re.captures(&name)
                .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_default()
        } else if is_app {
            let re = Regex::new(r"^([\s\S]*)@\d*?.service").unwrap();
            re.captures(&name)
                .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_default()
        } else {
            String::new()
        };

        debug!(
            "ServiceGObject::new: name: {name}, display_name: {display_name}, \
                is_vm: {is_vm}, is_app: {is_app}, vm_name: {vm_name:?}, \
                details: {details}, status: {status}",
        );

        let has_wireguard = is_vm && vm_name.as_ref().is_some_and(|s| static_contains(s));

        Object::builder()
            .property("name", name)
            .property("display-name", display_name)
            .property("is-vm", is_vm)
            .property("is-app", is_app)
            .property("vm-name", vm_name.unwrap_or_default())
            .property("details", details)
            //for demo
            .property("status", status)
            .property("trust-level", trust_level.into()) //trust_level as u8)
            .property("has-wireguard", has_wireguard)
            .build()
    }

    pub fn is_equal_to(&self, other: &ServiceGObject) -> bool {
        self.name() == other.name()
    }

    pub fn update(&self, query_result: QueryResult) {
        self.set_property("details", query_result.description);
        self.set_property("status", query_result.status as u8);
        //for demo
        //self.set_property("trust-level", query_result.trust_level as u8);
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
            ..
        }: QueryResult,
    ) -> Self {
        Self::new(
            name,
            description,
            status,
            trust_level,
            service_type,
            vm_name,
        )
    }
}
