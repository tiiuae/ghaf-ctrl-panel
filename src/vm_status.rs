use givc_common::query::VMStatus as GivcVMStatus;
use glib::Enum;
use gtk::glib;

// Derive necessary traits for automatic conversion
#[derive(
    Debug, Default, Clone, Copy, Enum, PartialEq, Eq, strum::IntoStaticStr, strum::Display,
)]
#[enum_type(name = "VMStatus")]
#[repr(u8)] // Optional: Ensure each variant has a specific discriminant value
pub enum VMStatus {
    Running = 0,
    #[default]
    #[strum(serialize = "Powered off")]
    PoweredOff = 1,
    Paused = 2,
}

impl From<GivcVMStatus> for VMStatus {
    fn from(status: GivcVMStatus) -> Self {
        match status {
            GivcVMStatus::Running => Self::Running,
            GivcVMStatus::PoweredOff => Self::PoweredOff,
            GivcVMStatus::Paused => Self::Paused,
        }
    }
}

impl VMStatus {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Running => "/org/gnome/controlpanelgui/icons/ellipse_green.svg",
            Self::PoweredOff => "/org/gnome/controlpanelgui/icons/ellipse_yellow.svg",
            Self::Paused => "/org/gnome/controlpanelgui/icons/ellipse_red.svg",
        }
    }
}
