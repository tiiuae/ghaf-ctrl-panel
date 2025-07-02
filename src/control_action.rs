use gtk::glib;

#[derive(Debug, Clone, Copy, glib::Enum)]
#[enum_type(name = "CtrlControlAction")]
#[repr(u8)]
pub enum ControlAction {
    Start,
    Restart,
    Pause,
    Resume,
    Shutdown,
}
