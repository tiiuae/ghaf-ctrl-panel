use glib::subclass::Signal;
use glib::Binding;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate};
use std::cell::RefCell;
use std::sync::OnceLock;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/language_region_notify_popup.ui")]
    pub struct LanguageRegionNotifyPopup {
        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LanguageRegionNotifyPopup {
        const NAME: &'static str = "LanguageRegionNotifyPopup";
        type Type = super::LanguageRegionNotifyPopup;
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
    impl LanguageRegionNotifyPopup {
        #[template_callback]
        fn on_ok_clicked(&self) {
            self.obj().close();
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for LanguageRegionNotifyPopup {
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
    impl WidgetImpl for LanguageRegionNotifyPopup {}
    impl BoxImpl for LanguageRegionNotifyPopup {}
    impl WindowImpl for LanguageRegionNotifyPopup {}
}

glib::wrapper! {
pub struct LanguageRegionNotifyPopup(ObjectSubclass<imp::LanguageRegionNotifyPopup>)
@extends gtk::Window, gtk::Widget, @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for LanguageRegionNotifyPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageRegionNotifyPopup {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn init(&self) {}
}
