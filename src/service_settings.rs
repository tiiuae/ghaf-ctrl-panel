use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::service_gobject::ServiceGObject;
use crate::trust_level::TrustLevel;
use crate::vm_status::VMStatus;

mod imp {
    use glib::subclass::Signal;
    use glib::{Binding, Variant};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{
        glib, Button, CompositeTemplate, Image, Label, MenuButton, Popover, Revealer, Separator,
        ToggleButton,
    };
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use crate::audio_settings::AudioSettings;
    use crate::control_action::ControlAction;
    use crate::prelude::*;
    use crate::security_icon::SecurityIcon;
    use crate::service_gobject::ServiceGObject;
    use crate::settings_action::SettingsAction;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/service_settings.ui")]
    pub struct ServiceSettings {
        #[template_child]
        pub name_slot_1: TemplateChild<Label>,
        #[template_child]
        pub name_slot_2: TemplateChild<Label>,
        #[template_child]
        pub status_label: TemplateChild<Label>,
        #[template_child]
        pub arrow_button: TemplateChild<ToggleButton>,
        #[template_child]
        pub revealer: TemplateChild<Revealer>,
        #[template_child]
        pub extra_info: TemplateChild<Label>,
        #[template_child]
        pub status_icon: TemplateChild<Image>,
        #[template_child]
        pub details_label: TemplateChild<Label>,
        #[template_child]
        pub security_icon: TemplateChild<SecurityIcon>,
        #[template_child]
        pub security_label: TemplateChild<Label>,
        #[template_child]
        pub wireguard_section_separator: TemplateChild<Separator>,
        #[template_child]
        pub wireguard_button: TemplateChild<Button>,
        #[template_child]
        pub audio_settings_box: TemplateChild<AudioSettings>,
        #[template_child]
        pub control_label: TemplateChild<Label>,
        #[template_child]
        pub action_menu_button: TemplateChild<MenuButton>,
        #[template_child]
        pub popover_menu: TemplateChild<Popover>,
        #[template_child]
        pub popover_menu_2: TemplateChild<Popover>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
        pub(super) service: RefCell<Option<ServiceGObject>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServiceSettings {
        const NAME: &'static str = "ServiceSettings";
        type Type = super::ServiceSettings;
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
    impl ServiceSettings {
        #[template_callback]
        fn on_wireguard_button_clicked(&self) {
            if let Some(vm) = self.service.borrow().clone() {
                debug!("Wireguard GUI will be launched...");
                self.obj().emit_by_name::<()>(
                    "settings-action",
                    &[&SettingsAction::OpenWireGuard { vm }],
                );
            }
        }

        #[template_callback]
        fn open_info(&self) {
            let value = self.arrow_button.is_active();
            self.revealer.set_reveal_child(value);
            if value {
                self.arrow_button.set_icon_name("pan-up-symbolic");
            } else {
                self.arrow_button.set_icon_name("pan-down-symbolic");
            }
        }

        fn emit_control_action(&self, action: ControlAction) {
            if let Some(vm) = self.service.borrow().clone() {
                self.obj()
                    .emit_by_name::<()>("control-action", &[&action, &vm]);
            }
            self.popover_menu.popdown();
        }

        #[template_callback]
        fn on_start_clicked(&self) {
            self.emit_control_action(ControlAction::Start);
        }

        #[template_callback]
        fn on_shutdown_clicked(&self) {
            self.emit_control_action(ControlAction::Shutdown);
            self.popover_menu_2.popdown();
        }

        #[template_callback]
        fn on_pause_clicked(&self) {
            self.emit_control_action(ControlAction::Pause);
            self.popover_menu_2.popdown();
        }

        #[allow(clippy::unused_self)]
        #[template_callback]
        fn on_mic_changed(&self, _value: i32) {
            //debug!("Mic changed: {}", value);
            //+ new action: VM mic
        }

        #[allow(clippy::unused_self)]
        #[template_callback]
        fn on_speaker_changed(&self, _value: i32) {
            //debug!("Speaker changed: {}", value);
            //+ new action: VM speaker
        }

        #[allow(clippy::unused_self)]
        #[template_callback]
        fn on_mic_volume_changed(&self, _value: Variant) {
            //debug!("Mic volume: {}", value);
            //+ new action: VM volume
        }

        #[allow(clippy::unused_self)]
        #[template_callback]
        fn on_speaker_volume_changed(&self, _value: Variant) {
            //debug!("Speaker volume: {}", value);
            //+ new action: VM volume
        }

        #[template_callback]
        fn on_open_advanced_audio_settings(&self) {
            let action = SettingsAction::OpenAdvancedAudioSettingsWidget;
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for ServiceSettings {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<[Signal; 6]> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                [
                    Signal::builder("control-action")
                        .param_types([ControlAction::static_type(), ServiceGObject::static_type()])
                        .build(),
                    Signal::builder("settings-action")
                        .param_types([SettingsAction::static_type()])
                        .build(),
                    Signal::builder("vm-mic-changed")
                        .param_types([u32::static_type()])
                        .build(),
                    Signal::builder("vm-speaker-changed")
                        .param_types([u32::static_type()])
                        .build(),
                    Signal::builder("vm-mic-volume-changed")
                        .param_types([u32::static_type()])
                        .build(),
                    Signal::builder("vm-speaker-volume-changed")
                        .param_types([u32::static_type()])
                        .build(),
                ]
            })
        }
    }
    impl WidgetImpl for ServiceSettings {}
    impl BoxImpl for ServiceSettings {}
}

glib::wrapper! {
pub struct ServiceSettings(ObjectSubclass<imp::ServiceSettings>)
    @extends gtk::Widget, gtk::Box,
    @implements gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ServiceSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceSettings {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    #[allow(clippy::too_many_lines)]
    pub fn bind(&self, object: &ServiceGObject) {
        //unbind previous ones
        self.unbind();
        //make new
        let name = self.imp().name_slot_1.get();
        let is_vm_or_app = object.is_vm() || object.is_app();
        let arrow_button = self.imp().arrow_button.get();
        let extra_info = self.imp().extra_info.get();
        let status = self.imp().status_label.get();
        let status_icon = self.imp().status_icon.get();
        let details = self.imp().details_label.get();
        let security_icon = self.imp().security_icon.get();
        let security_label = self.imp().security_label.get();
        let control_label = self.imp().control_label.get();
        let audio_settings_box = self.imp().audio_settings_box.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        //wireguard label/button
        self.imp()
            .wireguard_section_separator
            .set_visible(object.has_wireguard());
        self.imp()
            .wireguard_button
            .set_visible(object.has_wireguard());

        //action popover
        if object.is_vm() {
            self.imp()
                .action_menu_button
                .set_popover(Some(&self.imp().popover_menu.get()));
        } else {
            self.imp()
                .action_menu_button
                .set_popover(Some(&self.imp().popover_menu_2.get()));
        }

        *self.imp().service.borrow_mut() = Some(object.clone());

        if is_vm_or_app {
            let full_service_name = self.imp().name_slot_2.get();

            let name_binding = object
                .bind_property("display-name", &name, "label")
                .sync_create()
                .build();
            bindings.push(name_binding);

            let full_service_name_binding = object
                .bind_property("name", &full_service_name, "label")
                .sync_create()
                .build();
            bindings.push(full_service_name_binding);
        } else {
            let name_binding = object
                .bind_property("name", &name, "label")
                .sync_create()
                .build();
            bindings.push(name_binding);
        }

        //arrow/more info button visibilty
        let arrow_visibilty_binding = object
            .bind_property("is-app", &arrow_button, "visible")
            .flags(glib::BindingFlags::DEFAULT)
            .sync_create()
            .build();
        bindings.push(arrow_visibilty_binding);

        let extra_info_binding = object
            .bind_property("vm-name", &extra_info, "label")
            .sync_create()
            .transform_to(move |_, value: &str| Some(format!("The app is in the {value}")))
            .build();
        bindings.push(extra_info_binding);

        let status_binding = object
            .bind_property("status", &status, "label")
            .sync_create()
            .transform_to(move |_, status: VMStatus| Some::<&'static str>(status.into()))
            .build();
        bindings.push(status_binding);

        let status_icon_binding = object
            .bind_property("status", &status_icon, "resource")
            .sync_create()
            .transform_to(move |_, status: VMStatus| Some(status.icon()))
            .build();
        bindings.push(status_icon_binding);

        let details_binding = object
            .bind_property("details", &details, "label")
            .sync_create()
            .build();
        bindings.push(details_binding);

        let security_icon_binding = object
            .bind_property("trust-level", &security_icon, "trust-level")
            .sync_create()
            .build();
        bindings.push(security_icon_binding);

        let security_label_binding = object
            .bind_property("trust-level", &security_label, "label")
            .sync_create()
            .transform_to(move |_, trust_level: TrustLevel| match trust_level {
                TrustLevel::Secure => Some("Secure!"),
                TrustLevel::Warning => Some("Security warning!"),
                TrustLevel::NotSecure => Some("Security alert!"),
            })
            .build();
        bindings.push(security_label_binding);

        //change label
        let controls_title_binding = object
            .bind_property("is-vm", &control_label, "label")
            .sync_create()
            .transform_to(move |_, is_vm: bool| {
                Some(if is_vm {
                    "VM Controls"
                } else {
                    "Service Controls"
                })
            })
            .build();
        bindings.push(controls_title_binding);

        //hide audio settings for services and apps?
        let audio_settings_visibilty_binding = object
            .bind_property("is-vm", &audio_settings_box, "visible")
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
        self.imp().service.borrow_mut().take();
    }
}
