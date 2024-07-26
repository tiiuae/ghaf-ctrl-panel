use std::cell::RefCell;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Stack, ListBox, Label};
use glib::{Binding, ToValue};
use gtk::gio::ListStore;

//use crate::vm_gobject::VMGObject; will be used in the future
use crate::audio_settings::AudioSettings;
use crate::settings_gobject::SettingsGObject;
use crate::info_settings_page::InfoSettingsPage;
use crate::security_settings_page::SecuritySettingsPage;
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
        pub info_settings_page: TemplateChild<InfoSettingsPage>,
        #[template_child]
        pub security_settings_page: TemplateChild<SecuritySettingsPage>,
        #[template_child]
        pub wifi_settings_page: TemplateChild<WifiSettingsPage>,
        #[template_child]
        pub audio_settings_page: TemplateChild<AudioSettings>,

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
            if let Some(action_row) = row.downcast_ref::<adw::ActionRow>() {
                let name: Option<String> = action_row.property("name");
                if let Some(name) = name {
                    self.stack.set_visible_child_name(&name);
                } else {
                    println!("(No title)");
                }
            } else {
                println!("(Invalid row type)");
            }
        }
    }//end #[gtk::template_callbacks]

    impl ObjectImpl for Settings {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            // Setup
            let obj = self.obj();
            obj.init();
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
        if let Some(row) = self.imp().settings_list_box.row_at_index(0) {
            self.imp().settings_list_box.select_row(Some(&row));
        }
    }

    pub fn bind(&self, settings_object: &SettingsGObject) {
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
}

