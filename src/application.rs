use adw::subclass::prelude::*;
use gio::ListModel;
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::control_action::ControlAction;
use crate::data_gobject::DataGObject;
use crate::error_popup::ErrorPopup;
use crate::plot::Plot;
use crate::security_icon::SecurityIcon;
use crate::serie::Serie;
use crate::service_gobject::ServiceGObject;
pub use crate::service_model::StatsResponse;
use crate::settings_action::SettingsAction;
use crate::status_icon::StatusIcon;
use crate::volume_widget::VolumeWidget;
use givc_client::endpoint::TlsConfig;
use log::debug;

mod imp {
    use adw::{prelude::*, subclass::prelude::*};
    use gio::ListStore;
    use glib::Properties;
    use gtk::CssProvider;
    use gtk::{gdk, gio, glib};
    use std::cell::RefCell;

    use crate::audio_control::AudioControl;
    use crate::connection_config::ConnectionConfig;
    use crate::data_gobject::DataGObject;
    use crate::language_region_notify_popup::LanguageRegionNotifyPopup;
    use crate::locale_provider::{LanguageRegionData, LocaleProvider};
    use crate::prelude::*;
    use crate::service_model::ServiceModel;

    use crate::ControlPanelGuiWindow;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ControlPanelGuiApplication)]
    pub struct ControlPanelGuiApplication {
        pub(super) service_model: ServiceModel,
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
                        } = LocaleProvider::get_timezone_locale_info().await;

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
        }
    }

    impl ControlPanelGuiApplication {
        fn load_css() {
            // Load the CSS file and add it to the provider
            let provider = CssProvider::new();
            provider.load_from_resource("/ae/tii/ghaf/controlpanelgui/styles/style.css");

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
                    let locset = app.imp().service_model.set_locale(locale);
                    if let Err(err) = locset.await {
                        warn!("Locale setting failed: {err}");
                    }
                    let tzset = app.imp().service_model.set_timezone(timezone);
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
            let show_config_action = Self::build_action("show-config", Self::show_config);
            let quit_action = Self::build_action("quit", Self::clean_n_quit);
            let about_action = Self::build_action("about", Self::show_about);
            self.obj()
                .add_action_entries([show_config_action, quit_action, about_action]);
        }

        fn show_config(&self) {
            let addr = self.service_model.address();
            let port = self.service_model.port().try_into().unwrap_or(0u16);
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
                        let port = values[2].get::<u32>().unwrap();
                        debug!("New config applied: address {addr}, port {port}");
                        app.imp().service_model.set_address(addr);
                        app.imp().service_model.set_port(port);
                        None
                    }
                ),
            );

            config.present();
        }

        fn clean_n_quit(&self) {
            self.obj().quit();
        }

        fn show_about(&self) {
            let window = self.obj().active_window().unwrap();
            let about = adw::AboutDialog::builder()
                .application_name("Ghaf Control Panel")
                .application_icon("ae.tii.ghaf./controlpanelgui")
                .developer_name("dmitry")
                .developers(vec!["dmitry"])
                .copyright("© 2024 dmitry")
                .build();

            about.present(Some(&window));
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
        let _ = StatusIcon::static_type();

        let app: Self = glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build();

        app.imp().service_model.set_address(addr);
        app.imp().service_model.set_port(u32::from(port));
        if let Some((addr, tls_info)) = tls_info {
            app.imp().service_model.set_tls_info(addr, tls_info);
        }

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
        self.imp().service_model.clone().upcast()
    }

    pub fn get_stats(
        &self,
        vm: String,
    ) -> impl std::future::Future<Output = Result<StatsResponse, anyhow::Error>> + use<'_> {
        self.imp().service_model.get_stats(vm)
    }

    pub fn control_service(&self, action: ControlAction, object: ServiceGObject) {
        debug!("Control service {name}, {action:?}", name = object.name());
        glib::spawn_future_local(glib::clone!(
            #[strong(rename_to = app)]
            self,
            async move {
                match action {
                    ControlAction::Start => app
                        .imp()
                        .service_model
                        .start_service(object)
                        .await
                        .map(|_| ()),
                    ControlAction::Restart => app
                        .imp()
                        .service_model
                        .restart_service(&object)
                        .await
                        .map(|_| ()),
                    ControlAction::Pause => app.imp().service_model.pause_service(&object).await,
                    ControlAction::Resume => app.imp().service_model.resume_service(&object).await,
                    ControlAction::Shutdown => app.imp().service_model.stop_service(&object).await,
                }
            }
        ));
    }

    fn open_wireguard(&self, vm: &ServiceGObject) {
        debug!("wireguard vm {vm}", vm = vm.name()); // Output: business-vm

        if vm.is_vm() {
            let vm_name = vm.vm_name();
            debug!("wireguard app name {vm_name}"); // Output: business-vm
            glib::spawn_future_local(glib::clone!(
                #[strong(rename_to = app)]
                self,
                async move {
                    app.imp()
                        .service_model
                        .start_app_in_vm("wireguard-gui".into(), vm_name, vec![])
                        .await
                        .ok();
                }
            ));
        }
    }

    pub fn perform_setting_action(&self, action: SettingsAction) {
        debug!("Performing settings action... {action:?}");
        match action {
            SettingsAction::RegionNLanguage { locale, timezone } => {
                self.imp().set_locale_timezone(locale, timezone);
            }
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
                glib::spawn_future_local(glib::clone!(
                    #[strong(rename_to = app)]
                    self,
                    async move { app.imp().service_model.check_for_update().await }
                ));
            }
            SettingsAction::UpdateRequest => {
                self.imp().service_model.update_request();
            }
        }
    }
}
