use crate::service_gobject::ServiceGObject;
use gtk::glib;

#[derive(Debug, Clone, glib::Boxed)]
#[boxed_type(name = "SettingsAction")]
#[repr(u8)]
pub enum SettingsAction {
    RegionNLanguage {
        locale: String,
        timezone: String,
    },
    ShowErrorPopup {
        message: String,
    },
    OpenWireGuard {
        vm: ServiceGObject,
    },
    CheckForUpdateRequest,
    UpdateRequest,
}
