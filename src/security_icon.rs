#[derive(Debug, Clone, Copy)]
pub struct SecurityIcon(pub &'static str);

impl SecurityIcon {
    const OK: Self = Self("/org/gnome/controlpanelgui/icons/security_well.svg");
    const ATTENTION: Self = Self("/org/gnome/controlpanelgui/icons/security_attention.svg");
    const ALERT: Self = Self("/org/gnome/controlpanelgui/icons/security_alert.svg");

    pub fn new(trust_level: u8) -> Self {
        match trust_level {
            0 => SecurityIcon::OK,
            1 => SecurityIcon::ATTENTION,
            2 => SecurityIcon::ALERT,
            _ => SecurityIcon::OK,
        }
    }
}
