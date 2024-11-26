use adw::subclass::prelude::*;
use glib::Variant;
use gtk::prelude::*;
use gtk::CssProvider;
use gtk::{gdk, gio, glib};
use std::cell::RefCell;
use std::rc::Rc;

use crate::add_network_popup::AddNetworkPopup;
use crate::confirm_display_settings_popup::ConfirmDisplaySettingsPopup;
use crate::connection_config::ConnectionConfig;
use crate::control_action::ControlAction;
use crate::data_gobject::DataGObject;
use crate::data_provider::imp::{DataProvider, LanguageRegionData};
use crate::error_popup::ErrorPopup;
use crate::language_region_notify_popup::LanguageRegionNotifyPopup;
use crate::settings_action::SettingsAction;
use crate::ControlPanelGuiWindow;
use givc_client::endpoint::TlsConfig;

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

                let win = window.clone();
                glib::spawn_future_local(async move {
                    let LanguageRegionData {
                        languages,
                        current_language,
                        timezones,
                        current_timezone,
                    } = DataProvider::get_timezone_locale_info().await;
                    let index = current_language.and_then(|cur| {
                        languages
                            .iter()
                            .enumerate()
                            .find_map(|(idx, lang)| (lang.code == cur).then_some(idx))
                    });
                    win.set_locale_model(
                        languages.into_iter().map(DataGObject::from).collect(),
                        index,
                    );

                    let index = current_timezone.and_then(|cur| {
                        timezones
                            .iter()
                            .enumerate()
                            .find_map(|(idx, tz)| (tz.code == cur).then_some(idx))
                    });
                    win.set_timezone_model(
                        timezones.into_iter().map(DataGObject::from).collect(),
                        index,
                    );
                });
                window.upcast()
            };

            // Ask the window manager/compositor to present the window
            window.present();
            //connect on start
            application
                .imp()
                .data_provider
                .borrow()
                .establish_connection();
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
    pub fn new(
        application_id: &str,
        flags: &gio::ApplicationFlags,
        address: String,
        port: u16,
        tls_info: Option<(String, TlsConfig)>,
    ) -> Self {
        let _ = DataGObject::static_type();
        let app: Self = glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build();

        app.imp()
            .data_provider
            .borrow()
            .set_service_address(address, port);
        app.imp().data_provider.borrow().set_tls_info(tls_info);

        app
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
        self.add_action_entries([
            reconnect_action,
            show_config_action,
            quit_action,
            about_action,
        ]);
    }

    pub fn show_config(&self) {
        let window = self.active_window().unwrap();
        let address = self
            .imp()
            .data_provider
            .borrow()
            .get_current_service_address();
        let config = ConnectionConfig::new(address.0, address.1);
        config.set_transient_for(Some(&window));
        config.set_modal(true);

        let app = self.clone();

        config.connect_local("new-config-applied", false, move |values| {
            //the value[0] is self
            let addr = values[1].get::<String>().unwrap();
            let port = values[2].get::<u32>().unwrap();
            println!("New config applied: address {addr}, port {port}");
            app.imp()
                .data_provider
                .borrow()
                .reconnect(Some((addr, port as u16)));
            None
        });

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

    //reconnect with the same config (addr:port)
    pub fn reconnect(&self) {
        self.imp().data_provider.borrow().reconnect(None);
    }

    pub fn get_store(&self) -> gio::ListStore {
        self.imp().data_provider.borrow().get_store()
    }

    pub fn control_service(&self, action: ControlAction, name: String) {
        println!("Control service {name}, {:?}", action);
        match action {
            ControlAction::Start => self.imp().data_provider.borrow().start_service(name),
            ControlAction::Restart => self.imp().data_provider.borrow().restart_service(name),
            ControlAction::Pause => self.imp().data_provider.borrow().pause_service(name),
            ControlAction::Resume => self.imp().data_provider.borrow().resume_service(name),
            ControlAction::Shutdown => self.imp().data_provider.borrow().stop_service(name),
        }
    }

    pub fn perform_setting_action(&self, action: SettingsAction, value: Variant) {
        match action {
            SettingsAction::AddNetwork => todo!(),
            SettingsAction::RemoveNetwork => todo!(),
            SettingsAction::RegionNLanguage => {
                let (locale, timezone): (String, String) = value.get().unwrap();
                let app = self.clone();
                let popup = LanguageRegionNotifyPopup::new();
                popup.set_transient_for(self.active_window().as_ref());
                popup.set_modal(true);
                glib::spawn_future_local(async move {
                    if let Err(err) = app.imp().data_provider.borrow().set_locale(locale).await {
                        println!("Locale setting failed: {err}");
                    }
                    if let Err(err) = app
                        .imp()
                        .data_provider
                        .borrow()
                        .set_timezone(timezone)
                        .await
                    {
                        println!("Timezone setting failed: {err}");
                    }

                    popup.present();
                });
            }
            SettingsAction::DateNTime => todo!(),
            SettingsAction::MouseSpeed => todo!(),
            SettingsAction::KeyboardLayout => todo!(),
            SettingsAction::Speaker => todo!(),
            SettingsAction::SpeakerVolume => todo!(),
            SettingsAction::Mic => todo!(),
            SettingsAction::MicVolume => todo!(),
            SettingsAction::ShowAddNetworkPopup => {
                let app = self.clone();
                let window = self.active_window().unwrap();
                let popup = AddNetworkPopup::new();
                popup.set_transient_for(Some(&window));
                popup.set_modal(true);
                popup.connect_local("new-network", false, move |values| {
                    //the value[0] is self
                    let name = values[1].get::<String>().unwrap();
                    let security = values[2].get::<String>().unwrap();
                    let password = values[3].get::<String>().unwrap();
                    println!("New network: {name}, {security}, {password}");
                    app.imp()
                        .data_provider
                        .borrow()
                        .add_network(name, security, password);
                    None
                });
                popup.present();
            }
            SettingsAction::ShowAddKeyboardPopup => {}
            SettingsAction::ShowConfirmDisplaySettingsPopup => {
                //let app = self.clone();//center in, resize or scale might be needed
                let window = self.active_window().unwrap();
                let popup = ConfirmDisplaySettingsPopup::new();
                popup.set_transient_for(Some(&window));
                popup.set_modal(true);
                popup.connect_local("reset-default", false, move |_| {
                    if let Some(window) = window.downcast_ref::<ControlPanelGuiWindow>() {
                        window.restore_default_display_settings();
                    }
                    None
                });
                popup.present();
                popup.launch_close_timer(5);
            }
            SettingsAction::ShowErrorPopup => {
                if let Some(error) = value.str() {
                    let popup = ErrorPopup::new(error);
                    popup.set_transient_for(self.active_window().as_ref());
                    popup.set_modal(true);
                    popup.present();
                }
            }
        };
    }

    pub fn clean_n_quit(&self) {
        self.imp().data_provider.borrow().disconnect();
        drop(self.imp().data_provider.borrow());
        self.quit();
    }
}
