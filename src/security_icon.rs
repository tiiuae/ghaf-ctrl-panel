use gtk::glib;

use crate::trust_level::TrustLevel;

mod imp {
    use std::cell::Cell;

    use glib::Properties;
    use gtk::{glib, prelude::*, subclass::prelude::*, Image};

    use crate::trust_level::TrustLevel;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::SecurityIcon)]
    pub struct SecurityIcon {
        #[property(get, set = SecurityIcon::set_trust_level, construct, builder(TrustLevel::Warning))]
        trust_level: Cell<TrustLevel>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SecurityIcon {
        const NAME: &'static str = "SecurityIcon";
        type Type = super::SecurityIcon;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("securityicon");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SecurityIcon {
        fn constructed(&self) {
            let image = Image::new();
            image.set_visible(true);
            image.set_resource(Some(self.trust_level.get().icon()));
            self.obj().append(&image);
            self.parent_constructed();
        }
    }

    impl BoxImpl for SecurityIcon {}
    impl WidgetImpl for SecurityIcon {}

    impl SecurityIcon {
        fn set_trust_level(&self, trust_level: TrustLevel) {
            self.trust_level.set(trust_level);
            if let Some(image) = self.obj().first_child().and_downcast::<Image>() {
                image.set_resource(Some(trust_level.icon()));
            }
        }
    }
}

glib::wrapper! {
    pub struct SecurityIcon(ObjectSubclass<imp::SecurityIcon>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Buildable;
}

impl Default for SecurityIcon {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl SecurityIcon {
    pub fn new(trust_level: TrustLevel) -> Self {
        glib::Object::builder()
            .property("trust-level", trust_level)
            .build()
    }
}
