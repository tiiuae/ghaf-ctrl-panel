pub enum SecurityIcon {
    OK,
    Attention,
    Alert,
}

pub struct SecurityIconStruct {
}

impl SecurityIconStruct {
    const OK_VALUE: &'static str = "/org/gnome/controlpanelgui/icons/security_well.svg";
    const ATTENTION_VALUE: &'static str = "/org/gnome/controlpanelgui/icons/security_attention.svg";
    const ALERT_VALUE: &'static str = "/org/gnome/controlpanelgui/icons/security_alert.svg";
}
