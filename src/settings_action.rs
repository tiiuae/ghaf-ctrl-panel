use gtk::glib;
use gtk::prelude::*;
use glib::{value::{FromValue, ToValue, Value}, Type};

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SettingsAction {
    AddNetwork = 0,
    RemoveNetwork = 1,
    RegionNLanguage = 2,
    DateNTime = 3,
    MouseSpeed = 4,
    KeyboardLayout = 5,
    Speaker = 6,
    SpeakerVolume = 7,
    Mic = 8,
    MicVolume = 9,
    ShowAddNetworkPopup = 10,
    ShowAddKeyboardPopup = 11,
}

impl StaticType for SettingsAction {
    fn static_type() -> Type {
        u8::static_type()
    }
}

unsafe impl FromValue<'_> for SettingsAction {
    type Checker = glib::value::GenericValueTypeChecker<Self>;

    unsafe fn from_value(value: &Value) -> Self {
        match value.get::<u8>().unwrap() {
            0 => SettingsAction::AddNetwork,
            1 => SettingsAction::RemoveNetwork,
            2 => SettingsAction::RegionNLanguage,
            3 => SettingsAction::DateNTime,
            4 => SettingsAction::MouseSpeed,
            5 => SettingsAction::KeyboardLayout,
            6 => SettingsAction::Speaker,
            7 => SettingsAction::SpeakerVolume,
            8 => SettingsAction::Mic,
            9 => SettingsAction::MicVolume,
            10 => SettingsAction::ShowAddNetworkPopup,
            11 => SettingsAction::ShowAddKeyboardPopup,
            _ => panic!("Invalid SettingsAction value"),
        }
    }
}

impl ToValue for SettingsAction {
    fn to_value(&self) -> Value {
        let v = match self {
            SettingsAction::AddNetwork => 0u8,
            SettingsAction::RemoveNetwork => 1u8,
            SettingsAction::RegionNLanguage => 2u8,
            SettingsAction::DateNTime => 3u8,
            SettingsAction::MouseSpeed => 4u8,
            SettingsAction::KeyboardLayout => 5u8,
            SettingsAction::Speaker => 6u8,
            SettingsAction::SpeakerVolume => 7u8,
            SettingsAction::Mic => 8u8,
            SettingsAction::MicVolume => 9u8,
            SettingsAction::ShowAddNetworkPopup => 10u8,
            SettingsAction::ShowAddKeyboardPopup => 11u8,
        };
        v.to_value()
    }

    fn value_type(&self) -> Type {
        u8::static_type()
    }
}
