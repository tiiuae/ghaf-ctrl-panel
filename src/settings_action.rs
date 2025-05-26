use glib::{
    value::{FromValue, ToValue, Value},
    Type,
};
use gtk::glib;
use gtk::prelude::*;

#[derive(Debug, Clone, Copy, strum::FromRepr)]
#[repr(u8)]
pub enum SettingsAction {
    AddNetwork,
    RemoveNetwork,
    RegionNLanguage,
    DateNTime,
    MouseSpeed,
    KeyboardLayout,
    Speaker,
    SpeakerVolume,
    SpeakerMute,
    Mic,
    MicVolume,
    MicMute,
    ShowAddNetworkPopup,
    ShowAddKeyboardPopup,
    ShowConfirmDisplaySettingsPopup,
    ShowErrorPopup,
    OpenWireGuard,
    OpenAdvancedAudioSettingsWidget,
    CheckForUpdateRequest,
    UpdateRequest,
}

impl StaticType for SettingsAction {
    fn static_type() -> Type {
        u8::static_type()
    }
}

unsafe impl FromValue<'_> for SettingsAction {
    type Checker = glib::value::GenericValueTypeChecker<Self>;

    unsafe fn from_value(value: &Value) -> Self {
        value
            .get::<u8>()
            .ok()
            .and_then(SettingsAction::from_repr)
            .expect("Invalid SettingsAction value")
    }
}

impl ToValue for SettingsAction {
    fn to_value(&self) -> Value {
        Value::from(*self as u8)
    }

    fn value_type(&self) -> Type {
        u8::static_type()
    }
}
