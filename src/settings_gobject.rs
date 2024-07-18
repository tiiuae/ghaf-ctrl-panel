use std::cell::RefCell;
use gtk::glib::{self, Object, Properties};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::glib::subclass::prelude::*;

pub mod imp {
    use super::*;

    #[derive(Default)]
    pub struct SettingsData {
        pub memory_usage: u32,
        pub cpu_load: u32,
        pub network_load: u32,
        pub date: u32,//?
        pub time: i64,//in sec
        pub wifi_on: bool,
        pub wifi_name: String,
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::SettingsGObject)]
    pub struct SettingsGObject {
        #[property(name = "memory-usage", get, set, type = u32, member = memory_usage)]
        #[property(name = "cpu-load", get, set, type = u32, member = cpu_load)]
        #[property(name = "network-load", get, set, type = u32, member = network_load)]
        #[property(name = "time", get, set, type = i64, member = time)]
        #[property(name = "wifi-on", get, set, type = bool, member = wifi_on)]
        #[property(name = "wifi-name", get, set, type = String, member = wifi_name)]
        pub data: RefCell<SettingsData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SettingsGObject {
        const NAME: &'static str = "SettingsGObject";
        type Type = super::SettingsGObject;
        type ParentType = glib::Object;
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for SettingsGObject {}
}

glib::wrapper! {
    pub struct SettingsGObject(ObjectSubclass<imp::SettingsGObject>);
}

impl Default for SettingsGObject {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsGObject {
    pub fn new() -> Self {
        Object::builder()/*properties*/.build()
    }
}
