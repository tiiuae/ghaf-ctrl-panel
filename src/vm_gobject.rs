use std::cell::RefCell;
use gtk::glib::{self, Object, Properties};
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use givc_common::query::{QueryResult, VMStatus, TrustLevel}; //cannot be used as property!
//use crate::trust_level::TrustLevel as MyTrustLevel;//type is no recognised in #property

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct VMData {
        pub name: String,
        pub details: String,
        pub status: u8,
        pub trust_level: u8,
        //pub my_trust_level: MyTrustLevel,
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::VMGObject)]
    pub struct VMGObject {
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "details", get, set, type = String, member = details)]
        #[property(name = "status", get, set, type = u8, member = status)]
        #[property(name = "trust-level", get, set, type = u8, member = trust_level)]
        //#[property(name = "my-trust-level", get, set, type = MyTrustLevel, member = my_trust_level)]
        pub data: RefCell<VMData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VMGObject {
        const NAME: &'static str = "VMObject";
        type Type = super::VMGObject;
        type ParentType = glib::Object;
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for VMGObject {}
}

glib::wrapper! {
    pub struct VMGObject(ObjectSubclass<imp::VMGObject>);
}

impl VMGObject {
    pub fn new(name: String, details: String, status: VMStatus, trust_level: TrustLevel) -> Self {
        Object::builder()
            .property("name", name)
            .property("details", details)
            .property("status", status as u8)
            .property("trust-level", trust_level as u8)
            .build()
    }

    pub fn is_equal_to(&self, other: &VMGObject) -> bool {
        self.name() == other.name()
    }

    pub fn update(&self, query_result: QueryResult) {
        self.set_property("details", query_result.description);
        self.set_property("status", query_result.status as u8);
        self.set_property("trust-level", query_result.trust_level as u8);
    }
}
