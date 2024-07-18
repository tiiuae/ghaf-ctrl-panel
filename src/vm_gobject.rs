use std::cell::RefCell;
use gtk::glib::{self, Object, Properties};
use gtk::prelude::*;
use gtk::subclass::prelude::*;

//use givc_client::client::{VMStatus, TrustLevel}; cannot be used as property

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct VMData {
        pub name: String,
        pub details: String,
        pub status: u8,
        pub trust_level: u8,
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::VMGObject)]
    pub struct VMGObject {
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "details", get, set, type = String, member = details)]
        #[property(name = "status", get, set, type = u8, member = status)]
        #[property(name = "trust-level", get, set, type = u8, member = trust_level)]
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
    pub fn new(name: String, details: String, status: u8, trust_level: u8) -> Self {
        Object::builder()
            .property("name", name)
            .property("details", details)
            .property("status", status)
            .property("trust-level", trust_level)
            .build()
    }
}
