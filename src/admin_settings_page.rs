use std::cell::RefCell;
use std::sync::OnceLock;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Notebook};
use glib::Binding;
use glib::subclass::Signal;

use crate::settings_gobject::SettingsGObject;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/admin_settings_page.ui")]
    pub struct AdminSettingsPage {
        #[template_child]
        pub tab_widget: TemplateChild<Notebook>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AdminSettingsPage {
        const NAME: &'static str = "AdminSettingsPage";
        type Type = super::AdminSettingsPage;
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
    impl AdminSettingsPage {
        #[template_callback]
        fn on_update_clicked(&self) {
            println!("Update clicked!");
            self.obj().emit_by_name::<()>("update-request", &[]);
        }
    }//end #[gtk::template_callbacks]

    impl ObjectImpl for AdminSettingsPage {
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
                    Signal::builder("update-request")
                    //.param_types([u32::static_type()])
                    .build(),
                    ]
            })
        }
    }
    impl WidgetImpl for AdminSettingsPage {}
    impl BoxImpl for AdminSettingsPage {}
}

glib::wrapper! {
pub struct AdminSettingsPage(ObjectSubclass<imp::AdminSettingsPage>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for AdminSettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl AdminSettingsPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
    pub fn init(&self) {

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

