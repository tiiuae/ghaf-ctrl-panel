use givc_common::query::TrustLevel as GivcTrustLevel;
use glib::Enum;
use gtk::glib;

// Derive necessary traits for automatic conversion
#[derive(Debug, Clone, Copy, Enum, Eq, PartialEq)]
#[enum_type(name = "TrustLevel")]
#[repr(u8)] // Optional: Ensure each variant has a specific discriminant value
#[derive(Default)]
pub enum TrustLevel {
    Secure = 0,
    #[default]
    Warning = 1,
    NotSecure = 2,
}

impl From<GivcTrustLevel> for TrustLevel {
    fn from(level: GivcTrustLevel) -> Self {
        match level {
            GivcTrustLevel::Secure => Self::Secure,
            GivcTrustLevel::Warning => Self::Warning,
            GivcTrustLevel::NotSecure => Self::NotSecure,
        }
    }
}

impl TrustLevel {
    const OK: &'static str = "/org/gnome/controlpanelgui/icons/security_well.svg";
    const ATTENTION: &'static str = "/org/gnome/controlpanelgui/icons/security_attention.svg";
    const ALERT: &'static str = "/org/gnome/controlpanelgui/icons/security_alert.svg";

    pub fn icon(self) -> &'static str {
        match self {
            Self::Warning => Self::ATTENTION,
            Self::NotSecure => Self::ALERT,
            Self::Secure => Self::OK,
        }
    }
}
