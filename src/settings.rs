use gtk::gio::ListModel;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use glib::subclass::Signal;
    use glib::Binding;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate, ListBox, Stack};
    use std::cell::RefCell;
    use std::sync::OnceLock;

    //use crate::service_gobject::ServiceGObject; will be used in the future
    use crate::admin_settings_page::AdminSettingsPage;
    use crate::audio_device_gobject::AudioDeviceType;
    use crate::audio_settings::AudioSettings;
    use crate::control_action::ControlAction;
    use crate::language_region_settings_page::LanguageRegionSettingsPage;
    use crate::security_settings_page::SecuritySettingsPage;
    use crate::service_gobject::ServiceGObject;
    use crate::settings_action::SettingsAction;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/settings.ui")]
    pub struct Settings {
        #[template_child]
        pub list_box: TemplateChild<ListBox>,
        #[template_child]
        pub stack: TemplateChild<Stack>,
        #[template_child]
        pub admin_settings_page: TemplateChild<AdminSettingsPage>,
        #[template_child]
        pub security_settings_page: TemplateChild<SecuritySettingsPage>,
        #[template_child]
        pub audio_settings_page: TemplateChild<AudioSettings>,
        #[template_child]
        pub language_region_settings_page: TemplateChild<LanguageRegionSettingsPage>,
        //pub vm_model: RefCell<ListModel>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Settings {
        const NAME: &'static str = "Settings";
        type Type = super::Settings;
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
    impl Settings {
        #[template_callback]
        fn on_settings_row_selected(&self, row: &gtk::ListBoxRow) {
            self.stack
                .set_visible_child_name(row.widget_name().as_str());
        }

        #[template_callback]
        fn on_locale_timezone_default(&self) {
            let action = SettingsAction::RegionNLanguage {
                locale: "en_US.utf8".into(),
                timezone: "UTC".into(),
            };
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
            self.language_region_settings_page
                .locale_select_find(|obj| obj.name() == "en_US.utf8");
            self.language_region_settings_page
                .timezone_select_find(|obj| obj.name() == "UTC");
        }

        #[template_callback]
        fn on_locale_timezone_changed(&self, locale: String, timezone: String) {
            let action = SettingsAction::RegionNLanguage { locale, timezone };
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
        }

        #[template_callback]
        fn on_mic_changed(&self, id: i32, dev_type: AudioDeviceType) {
            let action = SettingsAction::Mic { id, dev_type };
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
        }

        #[template_callback]
        fn on_speaker_changed(&self, id: i32, dev_type: AudioDeviceType) {
            let action = SettingsAction::Speaker { id, dev_type };
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
        }

        #[template_callback]
        fn on_mic_volume_changed(&self, id: i32, dev_type: AudioDeviceType, volume: i32) {
            let action = SettingsAction::MicVolume {
                id,
                dev_type,
                volume,
            };
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
        }

        #[template_callback]
        fn on_mic_mute_changed(&self, id: i32, dev_type: AudioDeviceType, muted: bool) {
            let action = SettingsAction::MicMute {
                id,
                dev_type,
                muted,
            };
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
        }

        #[template_callback]
        fn on_speaker_volume_changed(&self, id: i32, dev_type: AudioDeviceType, volume: i32) {
            let action = SettingsAction::SpeakerVolume {
                id,
                dev_type,
                volume,
            };
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
        }

        #[template_callback]
        fn on_speaker_mute_changed(&self, id: i32, dev_type: AudioDeviceType, muted: bool) {
            let action = SettingsAction::SpeakerMute {
                id,
                dev_type,
                muted,
            };
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
        }

        #[template_callback]
        fn on_open_advanced_audio_settings(&self) {
            let action = SettingsAction::OpenAdvancedAudioSettingsWidget;
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
        }

        #[template_callback]
        fn on_check_for_update_request(&self) {
            let action = SettingsAction::CheckForUpdateRequest;
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
        }

        #[template_callback]
        fn on_update_request(&self) {
            let action = SettingsAction::UpdateRequest;
            self.obj().emit_by_name::<()>("settings-action", &[&action]);
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for Settings {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            // Setup
            let obj = self.obj();
            obj.init();
        }
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("vm-control-action")
                        .param_types([ControlAction::static_type(), ServiceGObject::static_type()])
                        .build(),
                    Signal::builder("settings-action")
                        .param_types([SettingsAction::static_type()])
                        .build(),
                ]
            })
        }
    }
    impl WidgetImpl for Settings {}
    impl BoxImpl for Settings {}
}

glib::wrapper! {
pub struct Settings(ObjectSubclass<imp::Settings>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_locale_model(&self, model: impl IsA<ListModel>, selected: Option<usize>) {
        self.imp()
            .language_region_settings_page
            .set_locale_model(model, selected);
    }

    pub fn set_timezone_model(&self, model: impl IsA<ListModel>, selected: Option<usize>) {
        self.imp()
            .language_region_settings_page
            .set_timezone_model(model, selected);
    }

    pub fn init(&self) {
        if let Some(row) = self.imp().list_box.row_at_index(0) {
            self.imp().list_box.select_row(Some(&row));
        }
    }

    pub fn set_audio_devices(&self, devices: ListModel) {
        self.imp().audio_settings_page.set_audio_devices(devices);
    }
}
