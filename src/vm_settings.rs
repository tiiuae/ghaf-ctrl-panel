use glib::subclass::Signal;
use glib::Binding;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Image, Label, MenuButton, Popover};
use std::cell::RefCell;
use std::sync::OnceLock;

use crate::audio_settings::AudioSettings;
use crate::security_icon::SecurityIcon;
use crate::vm_control_action::VMControlAction;
use crate::vm_gobject::VMGObject;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/vm_settings.ui")]
    pub struct VMSettings {
        #[template_child]
        pub name_slot_1: TemplateChild<Label>,
        #[template_child]
        pub name_slot_2: TemplateChild<Label>,
        #[template_child]
        pub vm_status_label: TemplateChild<Label>,
        #[template_child]
        pub vm_status_icon: TemplateChild<Image>,
        #[template_child]
        pub vm_details_label: TemplateChild<Label>,
        #[template_child]
        pub security_icon: TemplateChild<Image>,
        #[template_child]
        pub security_label: TemplateChild<Label>,
        #[template_child]
        pub audio_settings_box: TemplateChild<AudioSettings>,
        #[template_child]
        pub control_label: TemplateChild<Label>,
        #[template_child]
        pub vm_action_menu_button: TemplateChild<MenuButton>,
        #[template_child]
        pub popover_menu: TemplateChild<Popover>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VMSettings {
        const NAME: &'static str = "VMSettings";
        type Type = super::VMSettings;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl VMSettings {
        #[template_callback]
        fn on_vm_start_clicked(&self) {
            let name = self.name_slot_1.label();
            let vm_name = self.name_slot_2.label();
            self.obj().emit_by_name::<()>(
                "vm-control-action",
                &[&VMControlAction::Start, &name, &vm_name],
            );
            self.popover_menu.popdown();
        }
        #[template_callback]
        fn on_vm_shutdown_clicked(&self) {
            let name = self.name_slot_1.label();
            let vm_name = self.name_slot_2.label();
            self.obj().emit_by_name::<()>(
                "vm-control-action",
                &[&VMControlAction::Shutdown, &name, &vm_name],
            );
            self.popover_menu.popdown();
        }
        #[template_callback]
        fn on_vm_pause_clicked(&self) {
            let name = self.name_slot_1.label();
            let vm_name = self.name_slot_2.label();
            self.obj().emit_by_name::<()>(
                "vm-control-action",
                &[&VMControlAction::Pause, &name, &vm_name],
            );
            self.popover_menu.popdown();
        }
        #[template_callback]
        fn on_mic_changed(&self, value: u32) {
            println!("Mic changed: {}", value);
        }
        #[template_callback]
        fn on_speaker_changed(&self, value: u32) {
            println!("Speaker changed: {}", value);
        }
        #[template_callback]
        fn on_mic_volume_changed(&self, value: f64) {
            println!("Mic volume: {}", value);
        }
        #[template_callback]
        fn on_speaker_volume_changed(&self, value: f64) {
            println!("Speaker volume: {}", value);
            //send message to client mod via channel in DataProvider
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for VMSettings {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("vm-control-action")
                        .param_types([
                            VMControlAction::static_type(),
                            String::static_type(),
                            String::static_type(),
                        ])
                        .build(),
                    Signal::builder("vm-mic-changed")
                        .param_types([u32::static_type()])
                        .build(),
                    Signal::builder("vm-speaker-changed")
                        .param_types([u32::static_type()])
                        .build(),
                    Signal::builder("vm-mic-volume-changed")
                        .param_types([f64::static_type()])
                        .build(),
                    Signal::builder("vm-speaker-volume-changed")
                        .param_types([f64::static_type()])
                        .build(),
                ]
            })
        }
    }
    impl WidgetImpl for VMSettings {}
    impl BoxImpl for VMSettings {}
}

glib::wrapper! {
pub struct VMSettings(ObjectSubclass<imp::VMSettings>)
    @extends gtk::Widget, gtk::Box,
    @implements gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for VMSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl VMSettings {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn bind(&self, vm_object: &VMGObject) {
        //unbind previous ones
        self.unbind();
        //make new
        let name = self.imp().name_slot_1.get();
        let is_app_vm: bool = vm_object.property("is-app-vm");
        let status = self.imp().vm_status_label.get();
        let status_icon = self.imp().vm_status_icon.get();
        let details = self.imp().vm_details_label.get();
        let security_icon = self.imp().security_icon.get();
        let security_label = self.imp().security_label.get();
        let control_label = self.imp().control_label.get();
        let audio_settings_box = self.imp().audio_settings_box.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        if is_app_vm {
            let full_service_name = self.imp().name_slot_2.get();

            let name_binding = vm_object
                .bind_property("app-name", &name, "label")
                .sync_create()
                .build();
            bindings.push(name_binding);

            let full_service_name_binding = vm_object
                .bind_property("name", &full_service_name, "label")
                .sync_create()
                .build();
            bindings.push(full_service_name_binding);
        } else {
            let name_binding = vm_object
                .bind_property("name", &name, "label")
                .sync_create()
                .build();
            bindings.push(name_binding);
        };

        let status_binding = vm_object
            .bind_property("status", &status, "label")
            .sync_create()
            .transform_to(move |_, value: &glib::Value| {
                let status = value.get::<u8>().unwrap_or(0);
                match status {
                    //make struct like for icon?
                    0 => Some(glib::Value::from("Running")),
                    1 => Some(glib::Value::from("Powered off")),
                    2 => Some(glib::Value::from("Paused")),
                    _ => Some(glib::Value::from("Powered off")),
                }
            })
            .build();
        bindings.push(status_binding);

        let status_icon_binding = vm_object
            .bind_property("status", &status_icon, "resource")
            .sync_create()
            .transform_to(move |_, value: &glib::Value| {
                let status = value.get::<u8>().unwrap_or(0);
                match status {
                    //make struct like for icon?
                    0 => Some(glib::Value::from(
                        "/org/gnome/controlpanelgui/icons/ellipse_green.svg",
                    )),
                    1 => Some(glib::Value::from(
                        "/org/gnome/controlpanelgui/icons/ellipse_red.svg",
                    )),
                    2 => Some(glib::Value::from(
                        "/org/gnome/controlpanelgui/icons/ellipse_yellow.svg",
                    )),
                    _ => Some(glib::Value::from(
                        "/org/gnome/controlpanelgui/icons/ellipse_red.svg",
                    )),
                }
            })
            .build();
        bindings.push(status_icon_binding);

        let details_binding = vm_object
            .bind_property("details", &details, "label")
            .sync_create()
            .build();
        bindings.push(details_binding);

        let security_icon_binding = vm_object
            .bind_property("trust-level", &security_icon, "resource")
            .sync_create()
            .transform_to(move |_, value: &glib::Value| {
                let trust_level = value.get::<u8>().unwrap_or(0);
                Some(glib::Value::from(SecurityIcon::new(trust_level).0))
            })
            .build();
        bindings.push(security_icon_binding);

        let security_label_binding = vm_object
            .bind_property("trust-level", &security_label, "label")
            .sync_create()
            .transform_to(move |_, value: &glib::Value| {
                let trust_level = value.get::<u8>().unwrap_or(0);
                match trust_level {
                    //make struct like for icon?
                    0 => Some(glib::Value::from("Secure!")),
                    1 => Some(glib::Value::from("Security warning!")),
                    2 => Some(glib::Value::from("Security alert!")),
                    _ => Some(glib::Value::from("Secure!")),
                }
            })
            .build();
        bindings.push(security_label_binding);

        //change label
        let controls_title_binding = vm_object
            .bind_property("is-app-vm", &control_label, "label")
            .sync_create()
            .transform_to(move |_, value: &glib::Value| {
                let is_app_vm = value.get::<bool>().unwrap_or(false);
                if (is_app_vm) {
                    Some(glib::Value::from("VM Controls"))
                } else {
                    Some(glib::Value::from("Service Controls"))
                }
            })
            .build();
        bindings.push(controls_title_binding);

        //hide audio settings for services
        let audio_settings_visibilty_binding = vm_object
            .bind_property("is-app-vm", &audio_settings_box, "visible")
            .sync_create()
            .build();
        bindings.push(audio_settings_visibilty_binding);
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
        //clean name slot 2
        self.imp().name_slot_2.set_text("");
    }
}
