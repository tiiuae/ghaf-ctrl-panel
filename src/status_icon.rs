use givc_common::query::VMStatus;
use gtk::glib;

mod imp {
    use std::cell::Cell;

    use givc_common::query::VMStatus;
    use glib::Properties;
    use gtk::{Label, Orientation, Picture, glib, prelude::*, subclass::prelude::*};

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::StatusIcon)]
    pub struct StatusIcon {
        #[property(get, set = StatusIcon::set_vm_status, construct, builder(VMStatus::default()))]
        vm_status: Cell<VMStatus>,

        image: Picture,
        label: Label,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StatusIcon {
        const NAME: &'static str = "StatusIcon";
        type Type = super::StatusIcon;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("securityicon");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for StatusIcon {
        fn constructed(&self) {
            self.obj().set_orientation(Orientation::Horizontal);
            self.image.set_visible(true);
            self.image.set_size_request(8, 8);
            self.image.set_hexpand(false);
            self.image.set_vexpand(false);
            self.image.set_halign(gtk::Align::Center);
            self.image.set_valign(gtk::Align::Center);
            self.label.set_margin_start(5);
            self.label.set_visible(true);
            self.label.add_css_class("normal-text");
            self.obj().append(&self.image);
            self.obj().append(&self.label);
            self.parent_constructed();
        }
    }

    impl BoxImpl for StatusIcon {}
    impl WidgetImpl for StatusIcon {}

    impl StatusIcon {
        fn set_vm_status(&self, vm_status: VMStatus) {
            self.vm_status.set(vm_status);
            self.image.set_resource(Some(match vm_status {
                VMStatus::Running => "/ae/tii/ghaf/controlpanelgui/icons/ellipse_green.svg",
                VMStatus::PoweredOff => "/ae/tii/ghaf/controlpanelgui/icons/ellipse_yellow.svg",
                VMStatus::Paused => "/ae/tii/ghaf/controlpanelgui/icons/ellipse_red.svg",
            }));
            self.label.set_label(match vm_status {
                VMStatus::Running => "Running",
                VMStatus::PoweredOff => "Powered off",
                VMStatus::Paused => "Paused",
            });
        }
    }
}

glib::wrapper! {
    pub struct StatusIcon(ObjectSubclass<imp::StatusIcon>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for StatusIcon {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl StatusIcon {
    pub fn new(trust_level: VMStatus) -> Self {
        glib::Object::builder()
            .property("trust-level", trust_level)
            .build()
    }
}
