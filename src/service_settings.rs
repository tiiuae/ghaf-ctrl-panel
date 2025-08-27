use givc_common::types::VmType;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::service_gobject::ServiceGObject;
use crate::window::ControlPanelGuiWindow;

mod imp {
    use glib::subclass::Signal;
    use glib::{Binding, Variant};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{
        gio, glib, Button, CompositeTemplate, Label, MenuButton, Popover, Revealer, Separator,
        ToggleButton,
    };
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use crate::audio_settings::AudioSettings;
    use crate::control_action::ControlAction;
    use crate::plot::Plot;
    use crate::prelude::*;
    use crate::security_icon::SecurityIcon;
    use crate::serie::Serie;
    use crate::service_gobject::ServiceGObject;
    use crate::settings_action::SettingsAction;
    use crate::status_icon::StatusIcon;

    pub(super) struct CancelGuard(gio::Cancellable);

    impl Drop for CancelGuard {
        fn drop(&mut self) {
            self.0.cancel();
        }
    }

    impl From<gio::Cancellable> for CancelGuard {
        fn from(c: gio::Cancellable) -> Self {
            Self(c)
        }
    }

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/ae/tii/ghaf/controlpanelgui/ui/service_settings.ui")]
    pub struct ServiceSettings {
        #[template_child]
        pub resources_info_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub memory_plot: TemplateChild<Plot>,
        #[template_child]
        pub cpu_plot: TemplateChild<Plot>,
        #[template_child]
        pub name_slot_1: TemplateChild<Label>,
        #[template_child]
        pub name_slot_2: TemplateChild<Label>,
        #[template_child]
        pub arrow_button: TemplateChild<ToggleButton>,
        #[template_child]
        pub revealer: TemplateChild<Revealer>,
        #[template_child]
        pub extra_info: TemplateChild<Label>,
        #[template_child]
        pub status_icon: TemplateChild<StatusIcon>,
        #[template_child]
        pub details_label: TemplateChild<Label>,
        #[template_child]
        pub security_icon: TemplateChild<SecurityIcon>,
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

        #[template_child]
        pub cpu_sys_serie: TemplateChild<Serie>,
        #[template_child]
        pub cpu_user_serie: TemplateChild<Serie>,
        #[template_child]
        pub mem_used_serie: TemplateChild<Serie>,
        #[template_child]
        pub mem_needed_serie: TemplateChild<Serie>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
        pub(super) stats_cancel: RefCell<Option<CancelGuard>>,
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
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            // Setup
            self.obj().init();
        }

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

    pub fn init(&self) {
        self.imp()
            .cpu_plot
            .set_view(None, None, Some(0.0), Some(1.0));
        self.imp()
            .cpu_plot
            .set_label_format(|f| format!("{pct:.0}%", pct = f * 100.));

        self.imp().memory_plot.set_view(None, None, Some(0.0), None);
        self.imp()
            .memory_plot
            .set_label_format(|f| format!("{mb:.0} MB", mb = f / 1_048_576.));
    }

    #[allow(clippy::too_many_lines)]
    pub fn bind(&self, object: &ServiceGObject) {
        if self.imp().service.borrow().as_ref() == Some(object) {
            return;
        }
        //unbind previous ones
        self.unbind();
        //make new
        let name = self.imp().name_slot_1.get();
        let is_vm_or_app = object.is_vm() || object.is_app();
        let arrow_button = self.imp().arrow_button.get();
        let extra_info = self.imp().extra_info.get();
        let status_icon = self.imp().status_icon.get();
        let details = self.imp().details_label.get();
        let security_icon = self.imp().security_icon.get();
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
        self.imp().resources_info_box.set_visible(object.is_vm());

        self.imp()
            .action_menu_button
            .set_sensitive(object.vm_type() != VmType::AdmVM);

        //action popover
        if object.is_vm() {
            self.imp()
                .action_menu_button
                .set_popover(Some(&self.imp().popover_menu.get()));
            let c = gio::Cancellable::new();
            self.imp()
                .stats_cancel
                .borrow_mut()
                .replace(c.clone().into());
            #[allow(clippy::cast_precision_loss)]
            glib::spawn_future_local(gio::CancellableFuture::new(
                glib::clone!(
                    #[strong(rename_to = settings)]
                    self,
                    #[strong]
                    object,
                    async move {
                        let mut i = 1f32;
                        if let Some(win) = settings.root().and_downcast::<ControlPanelGuiWindow>() {
                            let stats = win.get_stats(object.vm_name());
                            while let Ok(stats) = stats.recv().await {
                                if let Some(process) = stats.process {
                                    settings.imp().cpu_user_serie.push(
                                        i,
                                        process.user_cycles as f32 / process.total_cycles as f32,
                                    );
                                    settings.imp().cpu_sys_serie.push(
                                        i,
                                        (process.user_cycles + process.sys_cycles) as f32
                                            / process.total_cycles as f32,
                                    );
                                }
                                if let Some(memory) = stats.memory {
                                    settings.imp().memory_plot.set_view(
                                        None,
                                        None,
                                        Some(0.0),
                                        Some(memory.total as f32),
                                    );
                                    settings
                                        .imp()
                                        .mem_used_serie
                                        .push(i, (memory.total - memory.free) as f32);
                                    settings
                                        .imp()
                                        .mem_needed_serie
                                        .push(i, (memory.total - memory.available) as f32);
                                }
                                i += 1.;
                            }
                        }
                    }
                ),
                c,
            ));
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

        let status_icon_binding = object
            .bind_property("status", &status_icon, "vm-status")
            .sync_create()
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

        self.imp().cpu_sys_serie.get().clear();
        self.imp().cpu_user_serie.get().clear();
        self.imp().mem_used_serie.get().clear();
        self.imp().mem_needed_serie.get().clear();

        self.imp().stats_cancel.borrow_mut().take();
    }
}
