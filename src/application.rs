
use std::cell::{Ref, RefCell};
use std::rc::Rc;
use gtk::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib, gdk};
use gtk::CssProvider;
use gtk::glib::closure_local;

use crate::ControlPanelGuiWindow;
use crate::data_provider::imp::DataProvider;
use crate::connection_config::ConnectionConfig;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct ControlPanelGuiApplication {
        pub data_provider: Rc<RefCell<DataProvider>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ControlPanelGuiApplication {
        const NAME: &'static str = "ControlPanelGuiApplication";
        type Type = super::ControlPanelGuiApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for ControlPanelGuiApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
            obj.set_accels_for_action("app.reconnect", &["<primary>r"]);
        }

        fn dispose(&self) {
            println!("App obj destroyed!");
            self.data_provider.borrow().disconnect();
            drop(self.data_provider.borrow());
        }
    }

    impl ApplicationImpl for ControlPanelGuiApplication {
        // We connect to the activate callback to create a window when the application
        // has been launched. Additionally, this callback notifies us when the user
        // tries to launch a "second instance" of the application. When they try
        // to do that, we'll just present any existing window.
        fn activate(&self) {
            let application = self.obj();
            //load CSS styles
            application.load_css();
            // Get the current window or create one if necessary
            let window = if let Some(window) = application.active_window() {
                window
            } else {
                let window = ControlPanelGuiWindow::new(&*application);
                window.upcast()
            };

            // Ask the window manager/compositor to present the window
            window.present();
        }
    }

    impl GtkApplicationImpl for ControlPanelGuiApplication {}
    impl AdwApplicationImpl for ControlPanelGuiApplication {}
}

glib::wrapper! {
    pub struct ControlPanelGuiApplication(ObjectSubclass<imp::ControlPanelGuiApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl ControlPanelGuiApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build()
    }

    fn load_css(&self) {
        // Load the CSS file and add it to the provider
        let provider = CssProvider::new();
        provider.load_from_resource("/org/gnome/controlpanelgui/styles/style.css");

        // Add the provider to the default screen
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    fn setup_gactions(&self) {
        let reconnect_action = gio::ActionEntry::builder("reconnect")
            .activate(move |app: &Self, _, _| app.reconnect())
            .build();
        let show_config_action = gio::ActionEntry::builder("show-config")
            .activate(move |app: &Self, _, _| app.show_config())
            .build();
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.clean_n_quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        self.add_action_entries([reconnect_action, show_config_action, quit_action, about_action]);
    }

    pub fn show_config(&self) {
        let window = self.active_window().unwrap();
        let config = ConnectionConfig::new();
        config.set_transient_for(Some(&window));
        config.set_modal(true);

        let app = self.clone();

        config.connect_closure(
            "new-config-aplied",
            false,
            closure_local!(move |addr: String, port: u32| {
                println!("Addr {addr}, port {port}");
                app.imp().data_provider.borrow().set_connection_config(addr, port);
            }),
        );

        config.present();
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutWindow::builder()
            .transient_for(&window)
            .application_name("Ghaf Control panel")
            .application_icon("org.gnome.controlpanelgui")
            .developer_name("dmitry")
            .developers(vec!["dmitry"])
            .copyright("© 2024 dmitry")
            .build();

        about.present();
    }

    pub fn reconnect(&self) {
        self.imp().data_provider.borrow().reconnect();
    }

    pub fn disconnect(&self) {
        self.imp().data_provider.borrow().disconnect();
    }

    pub fn get_store(&self) -> gio::ListStore{
        self.imp().data_provider.borrow().get_store()
    }

    pub fn start_vm(&self, vm_name: String) {
        println!("Start VM {vm_name}");
        self.imp().data_provider.borrow().start_vm(vm_name);
    }

    pub fn pause_vm(&self, vm_name: String) {
        println!("Pause VM {vm_name}");
        self.imp().data_provider.borrow().pause_vm(vm_name);
    }

    pub fn resume_vm(&self, vm_name: String) {
        println!("Resume VM {vm_name}");
        self.imp().data_provider.borrow().resume_vm(vm_name);
    }

    pub fn shutdown_vm(&self, vm_name: String) {
        println!("Shutdown VM {vm_name}");
        self.imp().data_provider.borrow().shutdown_vm(vm_name);
    }

    pub fn restart_vm(&self, vm_name: String) {
        println!("Restart VM {vm_name}");
        self.imp().data_provider.borrow().restart_vm(vm_name);
    }

    pub fn clean_n_quit(&self) {
        self.imp().data_provider.borrow().disconnect();
        drop(self.imp().data_provider.borrow());
        self.quit();
    }
}
