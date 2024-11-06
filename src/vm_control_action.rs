use glib::{
    value::{FromValue, ToValue, Value},
    Type,
};
use gtk::glib;
use gtk::prelude::*;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum VMControlAction {
    Start = 0,
    Restart = 1,
    Pause = 2,
    Resume = 3,
    Shutdown = 4,
}

impl StaticType for VMControlAction {
    fn static_type() -> Type {
        u8::static_type()
    }
}

unsafe impl FromValue<'_> for VMControlAction {
    type Checker = glib::value::GenericValueTypeChecker<Self>;

    unsafe fn from_value(value: &Value) -> Self {
        match value.get::<u8>().unwrap() {
            0 => VMControlAction::Start,
            1 => VMControlAction::Restart,
            2 => VMControlAction::Pause,
            3 => VMControlAction::Resume,
            4 => VMControlAction::Shutdown,
            _ => panic!("Invalid VMControlAction value"),
        }
    }
}

impl ToValue for VMControlAction {
    fn to_value(&self) -> Value {
        let v = match self {
            VMControlAction::Start => 0u8,
            VMControlAction::Restart => 1u8,
            VMControlAction::Pause => 2u8,
            VMControlAction::Resume => 3u8,
            VMControlAction::Shutdown => 4u8,
        };
        v.to_value()
    }

    fn value_type(&self) -> Type {
        u8::static_type()
    }
}
