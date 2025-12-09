use gtk::glib;

use givc_common::query::TrustLevel;

mod imp {
    use std::cell::Cell;

    use givc_common::query::TrustLevel;
    use glib::Properties;
    use gtk::{Image, Label, Orientation, glib, prelude::*, subclass::prelude::*};

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::SecurityIcon)]
    pub struct SecurityIcon {
        #[property(get, set = SecurityIcon::set_trust_level, construct, builder(TrustLevel::Warning))]
        trust_level: Cell<TrustLevel>,

        #[property(get, set, construct_only)]
        show_label: Cell<bool>,

        image: Image,
        label: Label,
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
            self.obj().set_orientation(Orientation::Horizontal);
            self.image.set_visible(true);
            self.label.set_visible(self.show_label.get());
            self.label.set_margin_start(5);
            self.obj().append(&self.image);
            self.obj().append(&self.label);
            self.parent_constructed();
        }
    }

    impl BoxImpl for SecurityIcon {}
    impl WidgetImpl for SecurityIcon {}

    impl SecurityIcon {
        fn set_trust_level(&self, trust_level: TrustLevel) {
            self.trust_level.set(trust_level);
            self.image.set_resource(Some(match trust_level {
                TrustLevel::Warning => "/ae/tii/ghaf/controlpanelgui/icons/security_attention.svg",
                TrustLevel::NotSecure => "/ae/tii/ghaf/controlpanelgui/icons/security_alert.svg",
                TrustLevel::Secure => "/ae/tii/ghaf/controlpanelgui/icons/security_well.svg",
            }));
            self.label.set_label(match trust_level {
                TrustLevel::Secure => "Secure!",
                TrustLevel::Warning => "Security warning!",
                TrustLevel::NotSecure => "Security alert!",
            });
        }
    }
}

glib::wrapper! {
    pub struct SecurityIcon(ObjectSubclass<imp::SecurityIcon>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
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
