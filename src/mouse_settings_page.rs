use std::cell::RefCell;
use std::sync::OnceLock;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Switch, Scale};
use glib::Binding;
use glib::subclass::Signal;

use crate::settings_gobject::SettingsGObject;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/mouse_settings_page.ui")]
    pub struct MouseSettingsPage {
        #[template_child]
        pub mouse_speed: TemplateChild<Scale>,
        #[template_child]
        pub button_switch: TemplateChild<Switch>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MouseSettingsPage {
        const NAME: &'static str = "MouseSettingsPage";
        type Type = super::MouseSettingsPage;
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
    impl MouseSettingsPage {
        #[template_callback]
        fn on_mouse_speed_changed(&self, scale: &Scale) {
            let value = scale.value();
            println!("Mouse speed changed: {}", value);
            self.obj().emit_by_name::<()>("mouse-speed-changed", &[&value]);
        }
        #[template_callback] 
        fn on_button_switch_state_changed(&self, value: bool) -> bool {
            println!("Mouse buttons switched: {}", value);
            self.obj().emit_by_name::<()>("mouse-buttons-changed", &[&value]);
            false//propagate event futher
        }
    }//end #[gtk::template_callbacks]

    impl ObjectImpl for MouseSettingsPage {
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
                    Signal::builder("mouse-speed-changed")
                    .param_types([f64::static_type()])
                    .build(),
                    Signal::builder("mouse-buttons-changed")
                    .param_types([bool::static_type()])
                    .build(),
                    ]
            })
        }
    }
    impl WidgetImpl for MouseSettingsPage {}
    impl BoxImpl for MouseSettingsPage {}
}

glib::wrapper! {
pub struct MouseSettingsPage(ObjectSubclass<imp::MouseSettingsPage>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for MouseSettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl MouseSettingsPage {
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

