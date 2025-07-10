use crate::audio_device_gobject::AudioDeviceType;
use crate::service_gobject::ServiceGObject;
use gtk::glib;

#[derive(Debug, Clone, glib::Boxed)]
#[boxed_type(name = "SettingsAction")]
#[repr(u8)]
pub enum SettingsAction {
    AddNetwork,
    RemoveNetwork,
    RegionNLanguage {
        locale: String,
        timezone: String,
    },
    Speaker {
        id: i32,
        dev_type: AudioDeviceType,
    },
    SpeakerVolume {
        id: i32,
        dev_type: AudioDeviceType,
        volume: i32,
    },
    SpeakerMute {
        id: i32,
        dev_type: AudioDeviceType,
        muted: bool,
    },
    Mic {
        id: i32,
        dev_type: AudioDeviceType,
    },
    MicVolume {
        id: i32,
        dev_type: AudioDeviceType,
        volume: i32,
    },
    MicMute {
        id: i32,
        dev_type: AudioDeviceType,
        muted: bool,
    },
    ShowAddNetworkPopup,
    ShowErrorPopup {
        message: String,
    },
    OpenWireGuard {
        vm: ServiceGObject,
    },
    OpenAdvancedAudioSettingsWidget,
    CheckForUpdateRequest,
    UpdateRequest,
}
