use std::cell::RefCell;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Switch, Image};
use glib::{Binding, ToValue};
//use gtk::gio::ListStore; will be needed for list of available networks

use crate::settings_gobject::SettingsGObject;
use crate::add_network_popup::AddNetworkPopup;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/wifi_settings_page.ui")]
    pub struct WifiSettingsPage {
        #[template_child]
        pub switch: TemplateChild<Switch>,
        #[template_child]
        pub state_indicator: TemplateChild<Image>,
        #[template_child]
        pub security_indicator: TemplateChild<Image>,
        #[template_child]
        pub signal_indicator: TemplateChild<Image>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WifiSettingsPage {
        const NAME: &'static str = "WifiSettingsPage";
        type Type = super::WifiSettingsPage;
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
    impl WifiSettingsPage {
        #[template_callback]
        fn on_switch_state_changed(&self, value: bool) -> bool {
            println!("Wifi switched: {}", value);
            false//propagate event futher
        }
        #[template_callback]
        fn on_add_clicked(&self) {
            //let window = self.active_window().unwrap();
            let popup = AddNetworkPopup::new();
            //popup.set_transient_for(Some(&window));
            //popup.set_modal(true);
            popup.present();
        }
    }//end #[gtk::template_callbacks]

    impl ObjectImpl for WifiSettingsPage {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            // Setup
            let obj = self.obj();
            obj.init();
        }
    }
    impl WidgetImpl for WifiSettingsPage {}
    impl BoxImpl for WifiSettingsPage {}
}

glib::wrapper! {
pub struct WifiSettingsPage(ObjectSubclass<imp::WifiSettingsPage>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for WifiSettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl WifiSettingsPage {
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