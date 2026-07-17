use crate::service_gobject::ServiceGObject;
use gtk::glib;
use secrecy::SecretString;

#[derive(Debug, Clone)]
pub enum UpdateServerAuthMode {
    Anonymous,
    UserPass {
        username: String,
        password: SecretString,
    },
    OAuth {
        token: SecretString,
    },
}

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
    CheckForUpdateRequest {
        reference: String,
        auth_mode: UpdateServerAuthMode,
        insecure: bool,
    },
    DownloadUpdateRequest {
        reference: String,
        auth_mode: UpdateServerAuthMode,
        insecure: bool,
    },
    UpdateRequest,
}
