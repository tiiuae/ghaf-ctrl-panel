use glib::subclass::Signal;
use glib::Binding;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, Button, CompositeTemplate, Label};
use std::cell::RefCell;
use std::sync::OnceLock;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/error_popup.ui")]
    pub struct ErrorPopup {
        #[template_child]
        pub message_to_user: TemplateChild<Label>,
        #[template_child]
        pub countdown_label: TemplateChild<Label>,
        #[template_child]
        pub ok_button: TemplateChild<Button>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ErrorPopup {
        const NAME: &'static str = "ErrorPopup";
        type Type = super::ErrorPopup;
        type ParentType = gtk::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl ErrorPopup {
        #[template_callback]
        fn on_ok_clicked(&self) {
            self.obj().close();
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for ErrorPopup {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            // Setup
            let obj = self.obj();
            obj.init();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("reset-default").build()])
        }
    }
    impl WidgetImpl for ErrorPopup {}
    impl BoxImpl for ErrorPopup {}
    impl WindowImpl for ErrorPopup {}
}

glib::wrapper! {
pub struct ErrorPopup(ObjectSubclass<imp::ErrorPopup>)
@extends gtk::Widget, gtk::Window, @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for ErrorPopup {
    fn default() -> Self {
        Self::new("Unknown error")
    }
}

impl ErrorPopup {
    pub fn new(message_to_user: &str) -> Self {
        let popup: Self = glib::Object::builder().build();
        popup.imp().message_to_user.set_label(message_to_user);
        popup
    }

    pub fn init(&self) {}
}
