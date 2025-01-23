use glib::{
    value::{FromValue, ToValue, Value},
    Type,
};
use gtk::glib::subclass::prelude::*;
use gtk::glib::{self, Object, Properties};
use gtk::prelude::*;
use std::cell::RefCell;

pub mod imp {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    #[repr(i32)]
    pub enum AudioDeviceType {
        Sink = 0,
        Source = 1,
        SinkInput = 2,
        SourceOutput = 3,
    }

    impl StaticType for AudioDeviceType {
        fn static_type() -> Type {
            i32::static_type()
        }
    }

    unsafe impl FromValue<'_> for AudioDeviceType {
        type Checker = glib::value::GenericValueTypeChecker<Self>;

        unsafe fn from_value(value: &Value) -> Self {
            match value.get::<i32>().unwrap() {
                0 => AudioDeviceType::Sink,
                1 => AudioDeviceType::Source,
                2 => AudioDeviceType::SinkInput,
                3 => AudioDeviceType::SourceOutput,
                _ => panic!("Invalid AudioDeviceType value"),
            }
        }
    }

    impl ToValue for AudioDeviceType {
        fn to_value(&self) -> Value {
            let v = match self {
                AudioDeviceType::Sink => 0i32,
                AudioDeviceType::Source => 1i32,
                AudioDeviceType::SinkInput => 2i32,
                AudioDeviceType::SourceOutput => 3i32,
            };
            v.to_value()
        }

        fn value_type(&self) -> Type {
            i32::static_type()
        }
    }

    #[derive(Default)]
    pub struct AudioDeviceData {
        id: i32,
        pub dev_type: i32,
        pub name: String,
        pub volume: i32,
        pub muted: bool,
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::AudioDeviceGObject)]
    pub struct AudioDeviceGObject {
        #[property(name = "id", get, type = i32, member = id)]
        #[property(name = "dev-type", get, set, type = i32, member = dev_type)]
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "volume", get, set, type = i32, member = volume)]
        #[property(name = "muted", get, set, type = bool, member = muted)]
        pub data: RefCell<AudioDeviceData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AudioDeviceGObject {
        const NAME: &'static str = "AudioDeviceGObject";
        type Type = super::AudioDeviceGObject;
        type ParentType = glib::Object;
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for AudioDeviceGObject {}
}

glib::wrapper! {
    pub struct AudioDeviceGObject(ObjectSubclass<imp::AudioDeviceGObject>);
}

impl Default for AudioDeviceGObject {
    fn default() -> Self {
        Self::new(0, 0, "default".to_string(), 0, false)
    }
}

impl AudioDeviceGObject {
    pub fn new(id: i32, dev_type: i32, name: String, volume: i32, muted: bool) -> Self {
        Object::builder()
            .property("id", id)
            .property("dev-type", dev_type)
            .property("name", name.clone())
            .property("volume", volume)
            .property("muted", muted)
            .build()
    }

    pub fn update(&self, dev_type: i32, name: String, volume: i32, muted: bool) {
        self.set_property("dev-type", dev_type);
        self.set_property("name", name);
        self.set_property("volume", volume);
        self.set_property("muted", muted);
    }
}
