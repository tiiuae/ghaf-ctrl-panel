use gtk::glib;

mod imp {
    use crate::glib::subclass::Signal;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate, Image, Switch};
    use std::sync::OnceLock;

    use crate::prelude::*;

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
        #[allow(clippy::unused_self)]
        #[template_callback]
        fn on_switch_state_changed(&self, value: bool) -> bool {
            debug!("Wifi switched: {value}");
            false //propagate event futher
        }
        #[template_callback]
        fn on_add_clicked(&self) {
            self.obj().emit_by_name::<()>("show-add-network-popup", &[]);
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for WifiSettingsPage {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();
        }
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("show-add-network-popup").build()])
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
}
