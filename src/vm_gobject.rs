use std::cell::RefCell;
use gtk::glib::{self, Object, Properties};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::glib::subclass::prelude::*;

//use crate::security_icon::SecurityIcon;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct VMData {
        pub name: String,
        pub details: String,
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::VMGObject)]
    pub struct VMGObject {
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "details", get, set, type = String, member = details)]
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
    pub fn new(name: String, details: String) -> Self {
        Object::builder()
            .property("name", name)
            .property("details", details)
            .build()
    }
}
