use glib::Enum;
use gtk::glib;

#[derive(Debug, Clone, Copy, Enum)]
#[enum_type(name = "CtrlControlAction")]
#[repr(u8)]
pub enum ControlAction {
    Start = 0,
    Restart = 1,
    Pause = 2,
    Resume = 3,
    Shutdown = 4,
    Monitor = 5,
}
