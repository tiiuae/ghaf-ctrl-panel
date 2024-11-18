use gtk::glib::{self, Object, Properties};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use regex::Regex;
use std::cell::RefCell;

use givc_common::query::{QueryResult, TrustLevel, VMStatus}; //cannot be used as property!
                                                             //use crate::trust_level::TrustLevel as MyTrustLevel;//type is no recognised in #property
use givc_common::types::ServiceType;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ServiceData {
        pub name: String,         //unique name, used as id
        pub display_name: String, //user-friendly name
        pub is_vm: bool,
        pub is_app: bool,
        pub details: String,
        pub status: u8,
        pub trust_level: u8,
        //pub my_trust_level: MyTrustLevel,
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ServiceGObject)]
    pub struct ServiceGObject {
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "display-name", get, set, type = String, member = display_name)]
        #[property(name = "is-vm", get, set, type = bool, member = is_vm)]
        #[property(name = "is-app", get, set, type = bool, member = is_app)]
        #[property(name = "details", get, set, type = String, member = details)]
        #[property(name = "status", get, set, type = u8, member = status)]
        #[property(name = "trust-level", get, set, type = u8, member = trust_level)]
        //#[property(name = "my-trust-level", get, set, type = MyTrustLevel, member = my_trust_level)]
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
        _status: VMStatus,
        _trust_level: TrustLevel,
        service_type: ServiceType,
    ) -> Self {
        let is_vm = service_type == ServiceType::VM;
        let is_app = service_type == ServiceType::App;

        let display_name = if is_vm {
            let re = Regex::new(r"^microvm@([^@-]+)-.+$").unwrap();
            re.captures(&name.clone())
                .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or(String::from(""))
        } else if is_app {
            let re = Regex::new(r"^([\s\S]*)@\d*?.service").unwrap();
            re.captures(&name.clone())
                .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or(String::from(""))
        } else {
            String::from("")
        };

        Object::builder()
            .property("name", name)
            .property("display-name", display_name)
            .property("is-vm", is_vm)
            .property("is-app", is_app)
            .property("details", details)
            //for demo
            .property("status", 0u8) //status as u8)
            .property("trust-level", 0u8) //trust_level as u8)
            .build()
    }

    pub fn is_equal_to(&self, other: &ServiceGObject) -> bool {
        self.name() == other.name()
    }

    pub fn update(&self, query_result: QueryResult) {
        self.set_property("details", query_result.description);
        //for demo
        //self.set_property("status", query_result.status as u8);
        //self.set_property("trust-level", query_result.trust_level as u8);
    }
}
