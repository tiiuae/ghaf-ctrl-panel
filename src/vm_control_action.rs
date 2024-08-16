use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use glib::{ParamSpec, ParamSpecInt, value::{FromValue, ToValue, Value}, Type, ParamFlags, Enum, ValueDelegate};

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum VMControlAction {
    Start = 0,
    Restart = 1,
    Pause = 2,
    Resume = 3,
    Shutdown = 4,
}

impl StaticType for VMControlAction {
    fn static_type() -> Type {
        u32::static_type()
    }
}

unsafe impl FromValue<'_> for VMControlAction {
    type Checker = glib::value::GenericValueTypeChecker<Self>;

    unsafe fn from_value(value: &Value) -> Self {
        match value.get::<u32>().unwrap() {
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
            VMControlAction::Start => 0u32,
            VMControlAction::Restart => 1u32,
            VMControlAction::Pause => 2u32,
            VMControlAction::Resume => 3u32,
            VMControlAction::Shutdown => 4u32,
        };
        v.to_value()
    }

    fn value_type(&self) -> Type {
        u32::static_type()
    }
}

impl VMControlAction {
    fn to_u32(self) -> u32 {
        match self {
            VMControlAction::Start => 0,
            VMControlAction::Restart => 1,
            VMControlAction::Pause => 2,
            VMControlAction::Resume => 3,
            VMControlAction::Shutdown => 4,
        }
    }

    fn from_u32(value: u32) -> Option<VMControlAction> {
        match value {
            0 => Some(VMControlAction::Start),
            1 => Some(VMControlAction::Restart),
            2 => Some(VMControlAction::Pause),
            3 => Some(VMControlAction::Resume),
            4 => Some(VMControlAction::Shutdown),
            _ => None,
        }
    }
}

