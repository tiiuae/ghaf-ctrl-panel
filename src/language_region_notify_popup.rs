use gtk::{gio, glib};

mod imp {
    use glib::Binding;
    use glib::subclass::Signal;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{CompositeTemplate, glib};
    use std::cell::RefCell;
    use std::sync::OnceLock;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/ae/tii/ghaf/controlpanelgui/ui/language_region_notify_popup.ui")]
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
    @extends gtk::Widget, gtk::Window, gtk::Box,
    @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
        gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
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
}
