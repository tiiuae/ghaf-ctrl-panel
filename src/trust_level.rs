use gtk::glib;
//use glib::prelude::*;
use glib::Enum;
//use strum::EnumString;

// Derive necessary traits for automatic conversion
#[derive(Debug, Clone, Copy, Enum)]
#[enum_type(name = "TrustLevel")]
#[repr(u8)] // Optional: Ensure each variant has a specific discriminant value
#[derive(Default)]
pub enum TrustLevel {
    #[default]
    Secure = 0,
    Warning = 1,
    NotSecure = 2,
}

/*impl StaticType for TrustLevel {
    fn static_type() -> Type {
        u8::static_type()
    }
}*/

/*unsafe impl<'a> FromValue<'a> for TrustLevel {
    type Checker = glib::value::GenericValueTypeChecker<Self>;

    unsafe fn from_value(value: &'a Value) -> Self {
        match value.get::<u8>().unwrap() {
            0 => TrustLevel::Secure,
            1 => TrustLevel::Warning,
            2 => TrustLevel::NotSecure,
            _ => TrustLevel::NotSecure,
        }
    }
}

// Implement glib::value::ToValue for TrustLevel
impl ToValue for TrustLevel {
    fn to_value(&self) -> Value {
        (*self as u8).to_value()
    }

    fn value_type(&self) -> Type {
        u8::static_type()
    }
}*/
