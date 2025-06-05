use adw::subclass::prelude::*;
use gio::ListModel;
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::add_network_popup::AddNetworkPopup;
use crate::confirm_display_settings_popup::ConfirmDisplaySettingsPopup;
use crate::control_action::ControlAction;
use crate::data_gobject::DataGObject;
pub use crate::data_provider::StatsResponse;
use crate::error_popup::ErrorPopup;
use crate::plot::Plot;
use crate::security_icon::SecurityIcon;
use crate::serie::Serie;
use crate::service_gobject::ServiceGObject;
use crate::settings_action::SettingsAction;
use crate::stats_window::StatsWindow;
use crate::volume_widget::VolumeWidget;
use crate::ControlPanelGuiWindow;
use givc_client::endpoint::TlsConfig;
use givc_common::address::EndpointAddress;
use log::debug;

mod imp {
    use adw::subclass::prelude::*;
    use gio::ListStore;
    use glib::Properties;
    use gtk::prelude::*;
    use gtk::CssProvider;
    use gtk::{gdk, gio, glib};
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::audio_control::AudioControl;
    use crate::connection_config::ConnectionConfig;
    use crate::data_gobject::DataGObject;
    use crate::data_provider::{DataProvider, LanguageRegionData};
    use crate::language_region_notify_popup::LanguageRegionNotifyPopup;
    use crate::prelude::*;
    use crate::ControlPanelGuiWindow;
    use givc_common::address::EndpointAddress;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ControlPanelGuiApplication)]
    pub struct ControlPanelGuiApplication {
        pub(super) data_provider: Rc<RefCell<DataProvider>>,
        pub(super) audio_control: RefCell<AudioControl>,

        #[property(get, set)]
        window: RefCell<Option<ControlPanelGuiWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ControlPanelGuiApplication {
        const NAME: &'static str = "ControlPanelGuiApplication";
        type Type = super::ControlPanelGuiApplication;
        type ParentType = adw::Application;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ControlPanelGuiApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            self.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
            obj.set_accels_for_action("app.reconnect", &["<primary>r"]);
        }

        fn dispose(&self) {
            debug!("App obj destroyed!");
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
            Self::load_css();
            // Get the current window or create one if necessary
            let window = if let Some(window) = application.window() {
                window
            } else {
                let window = ControlPanelGuiWindow::new(&*application);

                glib::spawn_future_local(glib::clone!(
                    #[strong]
                    window,
                    async move {
                        let LanguageRegionData {
                            languages,
                            current_language,
                            timezones,
                            current_timezone,
                        } = DataProvider::get_timezone_locale_info().await;

                        let index = current_language
                            .and_then(|cur| languages.iter().position(|lang| lang.code == cur));
                        let model: ListStore =
                            languages.into_iter().map(DataGObject::from).collect();
                        window.set_locale_model(model, index);

                        let index = current_timezone
                            .and_then(|cur| timezones.iter().position(|tz| tz.code == cur));
                        let model: ListStore =
                            timezones.into_iter().map(DataGObject::from).collect();
                        window.set_timezone_model(model, index);
                    }
                ));

                self.obj().set_window(&window);
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

    impl ControlPanelGuiApplication {
        fn load_css() {
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

        pub fn set_locale_timezone(&self, locale: String, timezone: String) {
            let popup = LanguageRegionNotifyPopup::new();
            popup.set_transient_for(self.obj().active_window().as_ref());
            popup.set_modal(true);
            glib::spawn_future_local(glib::clone!(
                #[strong(rename_to = app)]
                self.obj(),
                async move {
                    let locset = app.imp().data_provider.borrow().set_locale(locale);
                    if let Err(err) = locset.await {
                        warn!("Locale setting failed: {err}");
                    }
                    let tzset = app.imp().data_provider.borrow().set_timezone(timezone);
                    if let Err(err) = tzset.await {
                        warn!("Timezone setting failed: {err}");
                    }

                    popup.present();
                }
            ));
        }

        fn build_action<F: Fn(&Self) + 'static>(
            name: &str,
            cb: F,
        ) -> gio::ActionEntry<super::ControlPanelGuiApplication> {
            type App = super::ControlPanelGuiApplication;
            gio::ActionEntry::builder(name)
                .activate(move |app: &App, _, _| cb(app.imp()))
                .build()
        }

        fn setup_gactions(&self) {
            let reconnect_action = Self::build_action("reconnect", Self::reconnect);
            let show_config_action = Self::build_action("show-config", Self::show_config);
            let quit_action = Self::build_action("quit", Self::clean_n_quit);
            let about_action = Self::build_action("about", Self::show_about);
            self.obj().add_action_entries([
                reconnect_action,
                show_config_action,
                quit_action,
                about_action,
            ]);
        }

        //reconnect with the same config (addr:port)
        fn reconnect(&self) {
            self.data_provider.borrow().reconnect(None);
        }

        fn show_config(&self) {
            let address = self.data_provider.borrow().get_current_service_address();
            let EndpointAddress::Tcp { addr, port } = address else {
                error!("Unsupported endpoint address");
                return;
            };
            let config = ConnectionConfig::new(&addr, port);
            config.set_transient_for(self.obj().active_window().as_ref());
            config.set_modal(true);

            config.connect_local(
                "new-config-applied",
                false,
                glib::clone!(
                    #[strong(rename_to = app)]
                    self.obj(),
                    move |values| {
                        //the value[0] is self
                        let addr = values[1].get::<String>().unwrap();
                        let port: u16 = values[2].get::<u32>().unwrap().try_into().unwrap();
                        debug!("New config applied: address {addr}, port {port}");
                        app.imp()
                            .data_provider
                            .borrow()
                            .reconnect(Some((addr, port)));
                        None
                    }
                ),
            );

            config.present();
        }

        fn clean_n_quit(&self) {
            self.data_provider.borrow().disconnect();
            self.obj().quit();
        }

        fn show_about(&self) {
            let window = self.obj().active_window().unwrap();
            let about = adw::AboutWindow::builder()
                .transient_for(&window)
                .application_name("Ghaf Control Panel")
                .application_icon("org.gnome.controlpanelgui")
                .developer_name("dmitry")
                .developers(vec!["dmitry"])
                .copyright("© 2024 dmitry")
                .build();

            about.present();
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
        flags: gio::ApplicationFlags,
        addr: String,
        port: u16,
        tls_info: Option<(String, TlsConfig)>,
    ) -> Self {
        let _ = DataGObject::static_type();
        let _ = VolumeWidget::static_type();
        let _ = Plot::static_type();
        let _ = Serie::static_type();
        let _ = SecurityIcon::static_type();

        let app: Self = glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build();

        app.imp()
            .data_provider
            .borrow()
            .set_service_address(EndpointAddress::Tcp { addr, port });
        app.imp().data_provider.borrow().set_tls_info(tls_info);

        //test dbus service
        app.imp()
            .audio_control
            .borrow()
            .fetch_audio_devices(glib::clone!(
                #[strong]
                app,
                move |list| app.window().unwrap().set_audio_devices((*list).clone())
            ));

        app
    }

    pub fn get_model(&self) -> ListModel {
        self.imp().data_provider.borrow().get_model()
    }

    pub fn get_audio_devices(&self) -> ListModel {
        self.imp().audio_control.borrow().get_devices_list()
    }

    pub fn get_stats(
        &self,
        vm: String,
    ) -> impl std::future::Future<Output = Result<StatsResponse, String>> {
        self.imp().data_provider.borrow().get_stats(vm)
    }

    pub fn control_service(&self, action: ControlAction, object: ServiceGObject) {
        debug!("Control service {name}, {action:?}", name = object.name());
        match action {
            ControlAction::Start => self.imp().data_provider.borrow().start_service(object),
            ControlAction::Restart => self.imp().data_provider.borrow().restart_service(&object),
            ControlAction::Pause => self.imp().data_provider.borrow().pause_service(&object),
            ControlAction::Resume => self.imp().data_provider.borrow().resume_service(&object),
            ControlAction::Shutdown => self.imp().data_provider.borrow().stop_service(&object),
            ControlAction::Monitor => {
                let win = StatsWindow::new(object.vm_name());
                win.set_application(Some(self));
                win.set_visible(true);
            }
        }
    }

    fn show_add_network_popup(&self) {
        let popup = AddNetworkPopup::new();
        popup.set_transient_for(self.active_window().as_ref());
        popup.set_modal(true);
        popup.connect_local(
            "new-network",
            false,
            glib::clone!(
                #[strong(rename_to = app)]
                self,
                move |values| {
                    //the value[0] is self
                    let mut values = values.iter().skip(1).flat_map(glib::Value::get::<String>);
                    let name = values.next().unwrap();
                    let security = values.next().unwrap();
                    let password = values.next().unwrap();
                    debug!("New network: {name}, {security}, {password}");
                    app.imp()
                        .data_provider
                        .borrow()
                        .add_network(name, security, password);
                    None
                }
            ),
        );
        popup.present();
    }

    fn show_confirm_display_settings_popup(&self) {
        let window = self.active_window();
        let popup = ConfirmDisplaySettingsPopup::new();
        popup.set_transient_for(window.as_ref());
        popup.set_modal(true);
        popup.connect_local("reset-default", false, move |_| {
            if let Some(window) = window.and_downcast_ref::<ControlPanelGuiWindow>() {
                window.restore_default_display_settings();
            }
            None
        });
        popup.present();
        popup.launch_close_timer(5);
    }

    fn open_wireguard(&self, vm: &ServiceGObject) {
        debug!("wireguard vm {vm}", vm = vm.name()); // Output: business-vm

        if vm.is_vm() {
            let vm_name = vm.vm_name();
            debug!("wireguard app name {vm_name}"); // Output: business-vm
            self.imp().data_provider.borrow().start_app_in_vm(
                "wireguard-gui".into(),
                vm_name,
                vec![],
            );
        }
    }

    pub fn perform_setting_action(&self, action: SettingsAction) {
        match action {
            SettingsAction::AddNetwork => todo!(),
            SettingsAction::RemoveNetwork => todo!(),
            SettingsAction::RegionNLanguage { locale, timezone } => {
                self.imp().set_locale_timezone(locale, timezone);
            }
            SettingsAction::DateNTime => todo!(),
            SettingsAction::MouseSpeed => todo!(),
            SettingsAction::KeyboardLayout => todo!(),
            SettingsAction::Speaker { id, dev_type } => {
                debug!("Speaker changed: {id}");
                self.imp()
                    .audio_control
                    .borrow()
                    .set_default_device(id, dev_type as i32);
            }
            SettingsAction::SpeakerMute {
                id,
                dev_type,
                muted,
            } => {
                debug!("Speaker muted: {id}");
                self.imp()
                    .audio_control
                    .borrow()
                    .set_device_mute(id, dev_type as i32, muted);
            }
            SettingsAction::SpeakerVolume {
                id,
                dev_type,
                volume,
            } => {
                debug!("Speaker volume: {volume}");
                self.imp()
                    .audio_control
                    .borrow()
                    .set_device_volume(id, dev_type as i32, volume);
            }
            SettingsAction::Mic { id, dev_type } => {
                debug!("Mic changed: {id}");
                self.imp()
                    .audio_control
                    .borrow()
                    .set_default_device(id, dev_type as i32);
            }
            SettingsAction::MicMute {
                id,
                dev_type,
                muted,
            } => {
                debug!("Mic muted: {id}");
                self.imp()
                    .audio_control
                    .borrow()
                    .set_device_mute(id, dev_type as i32, muted);
            }
            SettingsAction::MicVolume {
                id,
                dev_type,
                volume,
            } => {
                debug!("Mic volume: {volume}");
                self.imp()
                    .audio_control
                    .borrow()
                    .set_device_volume(id, dev_type as i32, volume);
            }
            SettingsAction::ShowAddNetworkPopup => self.show_add_network_popup(),
            SettingsAction::ShowAddKeyboardPopup => todo!(),
            SettingsAction::ShowConfirmDisplaySettingsPopup => {
                self.show_confirm_display_settings_popup();
            }
            SettingsAction::ShowErrorPopup { message } => {
                let popup = ErrorPopup::new(&message);
                popup.set_transient_for(self.active_window().as_ref());
                popup.set_modal(true);
                popup.present();
            }
            SettingsAction::OpenWireGuard { vm } => {
                self.open_wireguard(&vm);
            }
            SettingsAction::OpenAdvancedAudioSettingsWidget => {
                self.imp()
                    .audio_control
                    .borrow()
                    .open_advanced_settings_widget();
            }
            SettingsAction::CheckForUpdateRequest => {
                self.imp().data_provider.borrow().check_for_update();
            }
            SettingsAction::UpdateRequest => {
                self.imp().data_provider.borrow().update_request();
            }
        }
    }
}
