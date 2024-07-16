use std::cell::RefCell;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, ListBox, Label};
use glib::{Binding, ToValue};

//use crate::vm_gobject::VMGObject; will be uesd in the future
//use crate::audio_settings::AudioSettings;
use crate::settings_gobject::SettingsGObject;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/settings.ui")]
    pub struct Settings {
        #[template_child]
        pub settings_list_box: TemplateChild<ListBox>,
        #[template_child]
        pub details_label: TemplateChild<Label>,

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
                let title: Option<String> = action_row.property("title");
                if let Some(title) = title {
                    self.details_label.set_text(&title);
                } else {
                    self.details_label.set_text("(No title)");
                }
            } else {
                self.details_label.set_text("(Invalid row type)");
            }
        }
    }//end #[gtk::template_callbacks]

    impl ObjectImpl for Settings {}
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

    /*pub fn bind(&self, settings_object: &SettingsGObject) {
        //unbind previous ones
        self.unbind();
        //make new
    }*/

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}

