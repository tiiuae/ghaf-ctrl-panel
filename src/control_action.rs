use glib::{
    value::{FromValue, ToValue, Value},
    Type,
};
use gtk::glib;
use gtk::prelude::*;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ControlAction {
    Start = 0,
    Restart = 1,
    Pause = 2,
    Resume = 3,
    Shutdown = 4,
}

impl StaticType for ControlAction {
    fn static_type() -> Type {
        u8::static_type()
    }
}

unsafe impl FromValue<'_> for ControlAction {
    type Checker = glib::value::GenericValueTypeChecker<Self>;

    unsafe fn from_value(value: &Value) -> Self {
        match value.get::<u8>().unwrap() {
            0 => ControlAction::Start,
            1 => ControlAction::Restart,
            2 => ControlAction::Pause,
            3 => ControlAction::Resume,
            4 => ControlAction::Shutdown,
            _ => panic!("Invalid ControlAction value"),
        }
    }
}

impl ToValue for ControlAction {
    fn to_value(&self) -> Value {
        let v = match self {
            ControlAction::Start => 0u8,
            ControlAction::Restart => 1u8,
            ControlAction::Pause => 2u8,
            ControlAction::Resume => 3u8,
            ControlAction::Shutdown => 4u8,
        };
        v.to_value()
    }

    fn value_type(&self) -> Type {
        u8::static_type()
    }
}
