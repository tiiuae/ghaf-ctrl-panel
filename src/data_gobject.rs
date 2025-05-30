use gtk::glib::{self, Object};

//use crate::trust_level::TrustLevel as MyTrustLevel;//type is no recognised in #property

mod imp {
    use gtk::glib::{self, Properties};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct DataData {
        pub name: String,
        pub display: String,
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::DataGObject)]
    pub struct DataGObject {
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "display", get, set, type = String, member = display)]
        pub data: RefCell<DataData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DataGObject {
        const NAME: &'static str = "DataGObject";
        type Type = super::DataGObject;
        type ParentType = glib::Object;
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for DataGObject {}
}

glib::wrapper! {
    pub struct DataGObject(ObjectSubclass<imp::DataGObject>);
}

impl DataGObject {
    pub fn new(name: String, display: String) -> Self {
        Object::builder()
            .property("name", name)
            .property("display", display)
            .build()
    }
}
