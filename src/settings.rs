use glib::subclass::Signal;
use glib::{Binding, Variant};
use gtk::gio::ListStore;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, ListBox, Stack};
use std::cell::RefCell;
use std::sync::OnceLock;

//use crate::vm_gobject::VMGObject; will be used in the future
use crate::admin_settings_page::AdminSettingsPage;
use crate::audio_settings::AudioSettings;
use crate::display_settings_page::DisplaySettingsPage;
use crate::info_settings_page::InfoSettingsPage;
use crate::keyboard_settings_page::KeyboardSettingsPage;
use crate::language_region_settings_page::LanguageRegionSettingsPage;
use crate::mouse_settings_page::MouseSettingsPage;
use crate::security_settings_page::SecuritySettingsPage;
use crate::settings_action::SettingsAction;
use crate::settings_gobject::SettingsGObject;
use crate::vm_control_action::VMControlAction;
use crate::wifi_settings_page::WifiSettingsPage;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/settings.ui")]
    pub struct Settings {
        #[template_child]
        pub settings_list_box: TemplateChild<ListBox>,
        #[template_child]
        pub stack: TemplateChild<Stack>,
        #[template_child]
        pub admin_settings_page: TemplateChild<AdminSettingsPage>,
        #[template_child]
        pub info_settings_page: TemplateChild<InfoSettingsPage>,
        #[template_child]
        pub security_settings_page: TemplateChild<SecuritySettingsPage>,
        #[template_child]
        pub wifi_settings_page: TemplateChild<WifiSettingsPage>,
        #[template_child]
        pub keyboard_settings_page: TemplateChild<KeyboardSettingsPage>,
        #[template_child]
        pub mouse_settings_page: TemplateChild<MouseSettingsPage>,
        #[template_child]
        pub audio_settings_page: TemplateChild<AudioSettings>,
        #[template_child]
        pub display_settings_page: TemplateChild<DisplaySettingsPage>,
        #[template_child]
        pub language_region_settings_page: TemplateChild<LanguageRegionSettingsPage>,

        //pub vm_model: RefCell<ListStore>,

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
        fn on_show_add_network_popup(&self) {
            let action = SettingsAction::ShowAddNetworkPopup;
            let empty = Variant::from(None::<()>.as_ref());
            self.obj()
                .emit_by_name::<()>("settings-action", &[&action, &empty]);
        }
        #[template_callback]
        fn on_show_add_new_keyboard_popup(&self) {
            let action = SettingsAction::ShowAddKeyboardPopup;
            let empty = Variant::from(None::<()>.as_ref());
            self.obj()
                .emit_by_name::<()>("settings-action", &[&action, &empty]);
        }

        #[template_callback]
        fn on_locale_timezone_changed(&self, locale: u32, timezone: u32) {
            let action = SettingsAction::RegionNLanguage;
            let variant = (
                match locale {
                    0 => "en_US.UTF-8",
                    1 => "ar_AE.UTF-8",
                    _ => return,
                },
                match timezone {
                    0 => "Asia/Abu_Dhabi",
                    1 => "Europe/Helsinki",
                    _ => return,
                },
            )
                .to_variant();
            self.obj()
                .emit_by_name::<()>("settings-action", &[&action, &variant]);
        }

        #[template_callback]
        fn on_show_confirm_display_settings_popup(&self) {
            let action = SettingsAction::ShowConfirmDisplaySettingsPopup;
            let empty = Variant::from(None::<()>.as_ref());
            self.obj()
                .emit_by_name::<()>("settings-action", &[&action, &empty]);
        }
        #[template_callback]
        fn on_show_error_popup(&self) {
            let action = SettingsAction::ShowErrorPopup;
            let message = Variant::from(String::from("Display settings cannot be set."));
            self.obj()
                .emit_by_name::<()>("settings-action", &[&action, &message]);
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
                        .param_types([VMControlAction::static_type(), String::static_type()])
                        .build(),
                    Signal::builder("settings-action")
                        .param_types([SettingsAction::static_type(), Variant::static_type()])
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

    pub fn set_vm_model(&self, model: ListStore) {
        self.imp().info_settings_page.set_vm_model(model.clone());
    }
    pub fn init(&self) {
        let this = self.clone();
        self.imp()
            .info_settings_page
            .connect_local("vm-control-action", false, move |values| {
                //the value[0] is self
                let vm_action = values[1].get::<VMControlAction>().unwrap();
                let vm_name = values[2].get::<String>().unwrap();
                this.emit_by_name::<()>("vm-control-action", &[&vm_action, &vm_name]);
                None
            });

        if let Some(row) = self.imp().settings_list_box.row_at_index(0) {
            self.imp().settings_list_box.select_row(Some(&row));
        }
    }

    pub fn bind(&self, _settings_object: &SettingsGObject) {
        //unbind previous ones
        self.unbind();
        //make new
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }

    pub fn restore_default_display_settings(&self) {
        self.imp().display_settings_page.restore_default();
    }
}
