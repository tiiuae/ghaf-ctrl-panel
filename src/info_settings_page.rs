use std::cell::RefCell;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Label, ProgressBar};
use glib::{Binding, ToValue};

use crate::vm_gobject::VMGObject;
use crate::settings_gobject::SettingsGObject;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/info_settings_page.ui")]
    pub struct InfoSettingsPage {
        #[template_child]
        pub memory_bar: TemplateChild<ProgressBar>,
        #[template_child]
        pub cpu_bar: TemplateChild<ProgressBar>,
        #[template_child]
        pub network_bar: TemplateChild<ProgressBar>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InfoSettingsPage {
        const NAME: &'static str = "InfoSettingsPage";
        type Type = super::InfoSettingsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
                klass.bind_template();
                //klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    /*#[gtk::template_callbacks]
    impl InfoSettingsPage {
        #[template_callback]
        fn on_row_selected(&self, row: &gtk::ListBoxRow) {
            
        }
    }*///end #[gtk::template_callbacks]

    impl ObjectImpl for InfoSettingsPage {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            // Setup
            let obj = self.obj();
            obj.init();
        }
    }
    impl WidgetImpl for InfoSettingsPage {}
    impl BoxImpl for InfoSettingsPage {}
}

glib::wrapper! {
pub struct InfoSettingsPage(ObjectSubclass<imp::InfoSettingsPage>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for InfoSettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl InfoSettingsPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
    pub fn init(&self) {
        //initial values to test styling
        self.imp().memory_bar.set_fraction(0.5);
        self.imp().cpu_bar.set_fraction(0.5);
        self.imp().network_bar.set_fraction(1.0);
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

