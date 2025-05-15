use gtk::gio;
use gtk::glib;

mod imp {
    use glib::subclass::Signal;
    use glib::Binding;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, Button, CompositeTemplate, Entry};
    use std::cell::RefCell;
    use std::sync::OnceLock;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/add_network_popup.ui")]
    pub struct AddNetworkPopup {
        #[template_child]
        pub name_entry: TemplateChild<Entry>,
        #[template_child]
        pub security_entry: TemplateChild<Entry>,
        #[template_child]
        pub password_entry: TemplateChild<Entry>,
        #[template_child]
        pub save_button: TemplateChild<Button>,
        #[template_child]
        pub cancel_button: TemplateChild<Button>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AddNetworkPopup {
        const NAME: &'static str = "AddNetworkPopup";
        type Type = super::AddNetworkPopup;
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
    impl AddNetworkPopup {
        #[template_callback]
        fn on_switch_state_changed(&self, value: bool) -> bool {
            self.password_entry.set_visibility(value);
            false //propagate event futher
        }
        #[template_callback]
        fn on_save_clicked(&self) {
            let name = self.name_entry.text().to_string();
            let sec = self.security_entry.text().to_string();
            let passwd = self.password_entry.text().to_string();
            self.obj()
                .emit_by_name::<()>("new-network", &[&name, &sec, &passwd]);
        }
        #[template_callback]
        fn on_cancel_clicked(&self) {
            self.obj().close();
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for AddNetworkPopup {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("new-network")
                    .param_types([
                        String::static_type(),
                        String::static_type(),
                        String::static_type(),
                    ])
                    .build()]
            })
        }
    }
    impl WidgetImpl for AddNetworkPopup {}
    impl BoxImpl for AddNetworkPopup {}
    impl WindowImpl for AddNetworkPopup {}
}

glib::wrapper! {
pub struct AddNetworkPopup(ObjectSubclass<imp::AddNetworkPopup>)
@extends gtk::Widget, gtk::Window, @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for AddNetworkPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl AddNetworkPopup {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}
