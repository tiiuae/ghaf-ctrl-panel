use gtk::glib::{self, Object};
use gtk::prelude::*;

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Default, glib::Enum, strum::FromRepr, strum::Display,
)]
#[enum_type(name = "AudioDeviceType")]
#[repr(i32)]
pub enum AudioDeviceType {
    #[default]
    Sink = 0,
    Source = 1,
    SinkInput = 2,
    SourceOutput = 3,
}

mod imp {
    use super::AudioDeviceType;
    use gtk::glib::subclass::prelude::*;
    use gtk::glib::{self, Properties};
    use gtk::prelude::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct AudioDeviceData {
        pub id: i32,
        pub dev_type: AudioDeviceType,
        pub name: String,
        pub volume: i32,
        pub muted: bool,
        pub is_default: bool,
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::AudioDeviceGObject)]
    pub struct AudioDeviceGObject {
        #[property(name = "id", get, set, type = i32, member = id)]
        #[property(name = "dev-type", get, set, type = AudioDeviceType, member = dev_type, builder(AudioDeviceType::Sink))]
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "volume", get, set, type = i32, member = volume)]
        #[property(name = "muted", get, set, type = bool, member = muted)]
        #[property(name = "is-default", get, set, type = bool, member = is_default)]
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
        Self::new(
            0,
            AudioDeviceType::Sink,
            "default".to_string(),
            0,
            false,
            false,
        )
    }
}

impl AudioDeviceGObject {
    pub fn new(
        id: i32,
        dev_type: AudioDeviceType,
        name: String,
        volume: i32,
        muted: bool,
        is_default: bool,
    ) -> Self {
        Object::builder()
            .property("id", id)
            .property("dev-type", dev_type)
            .property("name", name)
            .property("volume", volume)
            .property("muted", muted)
            .property("is-default", is_default)
            .build()
    }

    pub fn update(
        &self,
        dev_type: AudioDeviceType,
        name: String,
        volume: i32,
        muted: bool,
        is_default: bool,
    ) {
        self.set_property("dev-type", dev_type);
        self.set_property("name", name);
        self.set_property("volume", volume);
        self.set_property("muted", muted);
        self.set_property("is-default", is_default);
    }

    pub fn is_source(&self) -> bool {
        self.dev_type() == AudioDeviceType::Source
    }
}
