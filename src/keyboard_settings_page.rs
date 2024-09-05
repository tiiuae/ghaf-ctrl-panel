use std::cell::RefCell;
use std::sync::OnceLock;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, ListView, DropDown};
use glib::Binding;
use glib::subclass::Signal;

use crate::settings_gobject::SettingsGObject;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/keyboard_settings_page.ui")]
    pub struct KeyboardSettingsPage {
        #[template_child]
        pub region_language_switch: TemplateChild<DropDown>,
        #[template_child]
        pub keyboards_list_view: TemplateChild<ListView>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for KeyboardSettingsPage {
        const NAME: &'static str = "KeyboardSettingsPage";
        type Type = super::KeyboardSettingsPage;
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
    impl KeyboardSettingsPage {
        #[template_callback]
        fn on_add_clicked(&self) {
            println!("Add new keyboard!");
            self.obj().emit_by_name::<()>("show-add-new-keyboard-popup", &[]);
        }
    }//end #[gtk::template_callbacks]

    impl ObjectImpl for KeyboardSettingsPage {
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
                    Signal::builder("show-add-new-keyboard-popup")
                    .build(),
                    Signal::builder("remove-keyboard")
                    .param_types([u32::static_type()])
                    .build(),
                    ]
            })
        }
    }
    impl WidgetImpl for KeyboardSettingsPage {}
    impl BoxImpl for KeyboardSettingsPage {}
}

glib::wrapper! {
pub struct KeyboardSettingsPage(ObjectSubclass<imp::KeyboardSettingsPage>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for KeyboardSettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardSettingsPage {
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

