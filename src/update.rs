use adw::prelude::*;
use gtk::gio;
use gtk::glib;
use gtk::subclass::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use std::time::Duration;

use crate::app_config::{self, AppConfig, UpdateServerConfig};
use crate::application::ControlPanelGuiApplication;
use crate::prelude::*;
use crate::service_model::{ServiceModel, UpdateActivity, UpdateState};
use crate::settings_action::{SettingsAction, UpdateServerAuthMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WidgetState {
    Sensitive,
    Insensitive,
    Invisible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateServerRequestKind {
    Check,
    Download,
}

trait TemplateChildWidgetStateExt {
    fn set_widget_state(&self, state: WidgetState);
}

impl<T> TemplateChildWidgetStateExt for gtk::TemplateChild<T>
where
    T: gtk::prelude::ObjectType
        + gtk::glib::translate::FromGlibPtrNone<*mut <T as gtk::prelude::ObjectType>::GlibType>
        + IsA<gtk::Widget>,
{
    fn set_widget_state(&self, state: WidgetState) {
        let widget = self.get();
        match state {
            WidgetState::Sensitive => {
                widget.set_visible(true);
                widget.set_sensitive(true);
            }
            WidgetState::Insensitive => {
                widget.set_visible(true);
                widget.set_sensitive(false);
            }
            WidgetState::Invisible => {
                widget.set_visible(false);
                widget.set_sensitive(false);
            }
        }
    }
}

mod imp {
    use adw::prelude::*;
    use glib::SourceId;
    use glib::subclass::Signal;
    use gtk::subclass::prelude::*;
    use gtk::{
        Button, CheckButton, CompositeTemplate, DropDown, Entry, Image, Label, ProgressBar, Stack,
        TextBuffer, gio, glib,
    };
    use std::cell::{Cell, RefCell};
    use std::sync::OnceLock;

    use crate::prelude::*;
    use crate::service_model::UpdateActivity;
    use crate::settings_action::SettingsAction;

    pub(super) struct CancelGuard(gio::Cancellable);

    impl Drop for CancelGuard {
        fn drop(&mut self) {
            self.0.cancel();
        }
    }

    impl From<gio::Cancellable> for CancelGuard {
        fn from(cancellable: gio::Cancellable) -> Self {
            Self(cancellable)
        }
    }

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/ae/tii/ghaf/controlpanelgui/ui/update.ui")]
    pub struct UpdatePage {
        #[template_child]
        pub ghaf_logo: TemplateChild<Image>,
        #[template_child]
        pub update_stack: TemplateChild<Stack>,
        #[template_child]
        pub current_version: TemplateChild<Label>,
        #[template_child]
        pub available_version: TemplateChild<Label>,
        #[template_child]
        pub download_size: TemplateChild<Label>,
        #[template_child]
        pub changelog_buffer: TemplateChild<TextBuffer>,
        #[template_child]
        pub check_button: TemplateChild<Button>,
        #[template_child]
        pub download_button: TemplateChild<Button>,
        #[template_child]
        pub update_button: TemplateChild<Button>,
        #[template_child]
        pub operation_progress: TemplateChild<ProgressBar>,
        #[template_child]
        pub checking_box: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub no_updates_box: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub install_finished_box: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub update_details_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub update_server_address: TemplateChild<Entry>,
        #[template_child]
        pub update_server_insecure: TemplateChild<CheckButton>,
        #[template_child]
        pub update_server_auth_mode: TemplateChild<DropDown>,
        #[template_child]
        pub update_server_auth_stack: TemplateChild<Stack>,
        #[template_child]
        pub update_server_username: TemplateChild<Entry>,
        #[template_child]
        pub update_server_password: TemplateChild<Entry>,
        #[template_child]
        pub update_server_oauth_button: TemplateChild<Button>,
        pub(super) config_loading: Cell<bool>,
        pub(super) refresh_cancel: RefCell<Option<CancelGuard>>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        pub(super) activity_handler: RefCell<Option<glib::SignalHandlerId>>,
        pub(super) auth_mode_handler: RefCell<Option<glib::SignalHandlerId>>,
        pub(super) last_activity: RefCell<UpdateActivity>,
        pub(super) operation_progress_pulse: RefCell<Option<SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UpdatePage {
        const NAME: &'static str = "UpdatePage";
        type Type = super::UpdatePage;
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
    impl UpdatePage {
        #[template_callback]
        fn on_check_for_updates_clicked(&self) {
            let action = self.obj().build_check_for_update_action();
            self.obj()
                .emit_by_name::<()>("check-for-update-request", &[&action]);
        }

        #[template_callback]
        fn on_download_clicked(&self) {
            let action = self.obj().build_download_update_action();
            self.obj()
                .emit_by_name::<()>("download-update-request", &[&action]);
        }

        #[template_callback]
        fn on_update_clicked(&self) {
            self.obj().emit_by_name::<()>("install-update-request", &[]);
        }

        #[template_callback]
        fn on_start_oauth_clicked(&self) {
            if let Some(app) = self.obj().get_app_ref() {
                glib::spawn_future_local(glib::clone!(
                    #[strong(rename_to = model)]
                    app,
                    async move {
                        if let Err(err) = model
                            .get_service_model()
                            .start_update_server_oauth_flow()
                            .await
                        {
                            warn!("UpdatePage: failed to start OAuth flow: {err}");
                        }
                    }
                ));
            }
        }
    }

    impl ObjectImpl for UpdatePage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().load_persisted_update_server_settings();
            self.obj().connect_update_server_settings();
            self.obj().connect_root_notify(super::UpdatePage::refresh);
            self.obj()
                .connect_root_notify(super::UpdatePage::ensure_service_model_connection);
            self.obj().ensure_service_model_connection();
            self.obj().refresh();
        }

        fn dispose(&self) {
            if let Some(source_id) = self.operation_progress_pulse.borrow_mut().take() {
                source_id.remove();
            }
            self.refresh_cancel.borrow_mut().take();
            for binding in self.bindings.borrow_mut().drain(..) {
                binding.unbind();
            }
            if let Some(model) = self.obj().get_app_ref().map(|app| app.get_service_model()) {
                if let Some(handler_id) = self.activity_handler.borrow_mut().take() {
                    model.disconnect(handler_id);
                }
                if let Some(handler_id) = self.auth_mode_handler.borrow_mut().take() {
                    model.disconnect(handler_id);
                }
            }
            self.dispose_template();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("check-for-update-request")
                        .param_types([SettingsAction::static_type()])
                        .build(),
                    Signal::builder("download-update-request")
                        .param_types([SettingsAction::static_type()])
                        .build(),
                    Signal::builder("install-update-request").build(),
                ]
            })
        }
    }

    impl WidgetImpl for UpdatePage {}
    impl BoxImpl for UpdatePage {}
}

glib::wrapper! {
pub struct UpdatePage(ObjectSubclass<imp::UpdatePage>)
    @extends gtk::Widget, gtk::Box,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for UpdatePage {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdatePage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    fn collect_update_server_action(&self, action_kind: UpdateServerRequestKind) -> SettingsAction {
        let imp = self.imp();
        let auth_mode = match imp.update_server_auth_mode.selected() {
            1 => UpdateServerAuthMode::UserPass {
                username: imp.update_server_username.text().to_string(),
                password: SecretString::from(imp.update_server_password.text().to_string()),
            },
            2 => UpdateServerAuthMode::OAuth {
                token: SecretString::from(String::new()),
            },
            _ => UpdateServerAuthMode::Anonymous,
        };

        let reference = imp.update_server_address.text().trim().to_string();
        let insecure = imp.update_server_insecure.is_active();

        match action_kind {
            UpdateServerRequestKind::Check => SettingsAction::CheckForUpdateRequest {
                reference,
                auth_mode,
                insecure,
            },
            UpdateServerRequestKind::Download => SettingsAction::DownloadUpdateRequest {
                reference,
                auth_mode,
                insecure,
            },
        }
    }

    fn build_check_for_update_action(&self) -> SettingsAction {
        self.collect_update_server_action(UpdateServerRequestKind::Check)
    }

    fn build_download_update_action(&self) -> SettingsAction {
        self.collect_update_server_action(UpdateServerRequestKind::Download)
    }

    fn update_available(state: &UpdateState) -> bool {
        !state.available_version().is_empty()
    }

    fn reset_transient_update_ui(&self) {
        let imp = self.imp();
        imp.checking_box.set_visible(false);
        imp.no_updates_box.set_visible(false);
        imp.install_finished_box.set_visible(false);
        imp.update_details_box.set_visible(false);
        imp.operation_progress.set_visible(false);
        imp.download_button.set_visible(false);
        imp.update_button.set_visible(false);
    }

    fn set_idle_state(&self) {
        self.reset_transient_update_ui();
        let imp = self.imp();
        imp.check_button.set_widget_state(WidgetState::Sensitive);
    }

    fn set_checking_state(&self) {
        self.reset_transient_update_ui();
        let imp = self.imp();
        imp.checking_box.set_visible(true);
        imp.check_button.set_widget_state(WidgetState::Insensitive);
        imp.operation_progress
            .set_widget_state(WidgetState::Sensitive);
        imp.operation_progress.pulse();
        imp.operation_progress.set_fraction(0.0);
        imp.operation_progress.set_text(Some("Checking"));
    }

    fn set_checked_state(&self, update_available: bool) {
        let imp = self.imp();
        imp.check_button.set_widget_state(if update_available {
            WidgetState::Invisible
        } else {
            WidgetState::Sensitive
        });
        imp.no_updates_box.set_visible(!update_available);
        imp.update_details_box.set_visible(update_available);
        imp.download_button.set_visible(update_available);
    }

    fn set_downloading_state(&self, update_available: bool, progress: f64) {
        let imp = self.imp();
        imp.check_button.set_widget_state(WidgetState::Invisible);
        imp.update_details_box.set_visible(update_available);
        imp.operation_progress.set_visible(true);
        imp.operation_progress
            .set_widget_state(WidgetState::Sensitive);
        imp.operation_progress
            .set_fraction(progress.clamp(0.0, 1.0));
        imp.operation_progress
            .set_text(Some(&progress_text(progress)));
        imp.download_button
            .set_widget_state(WidgetState::Insensitive);
    }

    fn set_downloaded_state(&self, update_available: bool) {
        let imp = self.imp();
        imp.check_button.set_widget_state(WidgetState::Invisible);
        imp.update_details_box.set_visible(update_available);
        imp.update_button.set_visible(update_available);
    }

    fn set_installing_state(&self, update_available: bool) {
        let imp = self.imp();
        imp.check_button.set_widget_state(WidgetState::Invisible);
        imp.update_details_box.set_visible(update_available);
        imp.operation_progress.set_visible(true);
        imp.operation_progress
            .set_widget_state(WidgetState::Sensitive);
        imp.operation_progress.set_fraction(0.0);
        imp.operation_progress.set_text(Some("Installing"));
        self.start_operation_progress_activity();
        imp.update_button.set_widget_state(WidgetState::Insensitive);
    }

    fn set_installed_state(&self) {
        let imp = self.imp();
        self.reset_transient_update_ui();
        imp.check_button.set_widget_state(WidgetState::Invisible);
        imp.install_finished_box.set_visible(true);
    }

    fn stop_operation_progress_activity(&self) {
        if let Some(source_id) = self.imp().operation_progress_pulse.borrow_mut().take() {
            source_id.remove();
        }
    }

    fn start_operation_progress_activity(&self) {
        let imp = self.imp();
        if imp.operation_progress_pulse.borrow().is_some() {
            return;
        }

        let progress = imp.operation_progress.get();
        progress.pulse();
        let source_id = glib::timeout_add_local(Duration::from_millis(120), move || {
            progress.pulse();
            glib::ControlFlow::Continue
        });
        imp.operation_progress_pulse.borrow_mut().replace(source_id);
    }

    pub fn refresh(&self) {
        debug!("UpdatePage: refresh current version");
        if let Some(app) = self.get_app_ref() {
            let cancellable = gio::Cancellable::new();
            self.imp()
                .refresh_cancel
                .borrow_mut()
                .replace(cancellable.clone().into());
            glib::spawn_future_local(gio::CancellableFuture::new(
                glib::clone!(
                    #[weak(rename_to = page)]
                    self,
                    async move {
                        let status = fetch_system_status(&app).await;
                        app.get_service_model()
                            .set_update_current_version(status.ghaf_version);
                        page.apply_idle_state();
                    }
                ),
                cancellable,
            ));
        } else {
            warn!("UpdatePage: no app ref, cannot query host version");
            self.imp().current_version.set_label("unknown");
            self.apply_idle_state();
        }
    }

    fn ensure_service_model_connection(&self) {
        if self.imp().activity_handler.borrow().is_some() {
            return;
        }

        let Some(app) = self.get_app_ref() else {
            warn!("UpdatePage: no app ref, cannot subscribe to update state");
            return;
        };

        let model: ServiceModel = app.get_service_model();
        let update_state = model.current_update_state();
        self.bind_update_state_properties(&update_state);
        self.sync_state_controls(&update_state);

        {
            let handler_id = update_state.connect_notify_local(
                Some("activity"),
                glib::clone!(
                    #[weak(rename_to = page)]
                    self,
                    move |state, _| {
                        page.sync_state_controls(state);
                    }
                ),
            );
            self.imp().activity_handler.borrow_mut().replace(handler_id);
        }

        {
            let handler_id =
                self.imp()
                    .update_server_auth_mode
                    .connect_selected_notify(glib::clone!(
                        #[weak(rename_to = page)]
                        self,
                        move |dropdown| {
                            page.on_update_server_auth_mode_selected(dropdown.selected());
                        }
                    ));
            self.imp()
                .auth_mode_handler
                .borrow_mut()
                .replace(handler_id);
        }
    }

    fn on_update_server_auth_mode_selected(&self, selected: u32) {
        self.imp()
            .update_server_auth_stack
            .set_visible_child_name(match selected {
                1 => "userpass",
                2 => "oauth",
                _ => "anonymous",
            });
        self.persist_update_server_settings();
    }

    fn connect_update_server_settings(&self) {
        self.imp()
            .update_server_address
            .connect_changed(glib::clone!(
                #[weak(rename_to = page)]
                self,
                move |_| {
                    page.persist_update_server_settings();
                }
            ));

        self.imp()
            .update_server_insecure
            .connect_toggled(glib::clone!(
                #[weak(rename_to = page)]
                self,
                move |_| {
                    page.persist_update_server_settings();
                }
            ));

        self.imp()
            .update_server_username
            .connect_changed(glib::clone!(
                #[weak(rename_to = page)]
                self,
                move |_| {
                    page.persist_update_server_settings();
                }
            ));

        self.imp()
            .update_server_password
            .connect_changed(glib::clone!(
                #[weak(rename_to = page)]
                self,
                move |_| {
                    page.persist_update_server_settings();
                }
            ));
    }

    fn load_persisted_update_server_settings(&self) {
        let Ok(config) = app_config::load_config() else {
            return;
        };

        let imp = self.imp();
        imp.config_loading.set(true);
        imp.update_server_address.set_text(&config.update.reference);
        imp.update_server_insecure
            .set_active(config.update.insecure);
        imp.update_server_username.set_text(&config.update.username);
        imp.update_server_password
            .set_text(config.update.password.expose_secret());

        let selected = match config.update.auth_mode.as_str() {
            "anonymous" => 0,
            "oauth" => 2,
            _ => 1,
        };
        imp.update_server_auth_mode.set_selected(selected);
        self.on_update_server_auth_mode_selected(selected);
        imp.config_loading.set(false);
    }

    fn persist_update_server_settings(&self) {
        let imp = self.imp();
        if imp.config_loading.get() {
            return;
        }

        let mut config = match app_config::load_config() {
            Ok(config) => config,
            Err(err) => {
                warn!("UpdatePage: failed to load persisted config: {err}");
                AppConfig::default()
            }
        };

        config.update = UpdateServerConfig {
            auth_mode: match imp.update_server_auth_mode.selected() {
                0 => String::from("anonymous"),
                2 => String::from("oauth"),
                _ => String::from("user-pass"),
            },
            reference: imp.update_server_address.text().trim().to_string(),
            insecure: imp.update_server_insecure.is_active(),
            username: imp.update_server_username.text().to_string(),
            password: SecretString::from(imp.update_server_password.text().to_string()),
            oauth_token: config.update.oauth_token.clone(),
        };

        if let Err(err) = app_config::save_config(&config) {
            warn!("UpdatePage: failed to persist update server settings: {err}");
        }
    }

    fn bind_update_state_properties(&self, state: &UpdateState) {
        let imp = self.imp();
        let mut bindings = imp.bindings.borrow_mut();

        bindings.push(
            state
                .bind_property("current-version", &imp.current_version.get(), "label")
                .sync_create()
                .transform_to(|_, value: &str| {
                    Some(if value.is_empty() {
                        "unknown".to_string()
                    } else {
                        value.to_string()
                    })
                })
                .build(),
        );

        bindings.push(
            state
                .bind_property("available-version", &imp.available_version.get(), "label")
                .sync_create()
                .transform_to(|_, value: &str| {
                    Some(if value.is_empty() {
                        "Not checked".to_string()
                    } else {
                        value.to_string()
                    })
                })
                .build(),
        );

        bindings.push(
            state
                .bind_property("changelog", &imp.changelog_buffer.get(), "text")
                .sync_create()
                .build(),
        );

        bindings.push(
            state
                .bind_property("download-size-bytes", &imp.download_size.get(), "label")
                .sync_create()
                .transform_to(|_, bytes: u64| {
                    Some(if bytes == 0 {
                        "Download size: unknown".to_string()
                    } else {
                        format!("Download size: {}", format_bytes(bytes))
                    })
                })
                .build(),
        );
    }

    fn sync_state_controls(&self, state: &UpdateState) {
        let imp = self.imp();
        let activity = state.activity();
        let previous_activity = imp.last_activity.borrow().clone();
        self.stop_operation_progress_activity();
        let update_available = Self::update_available(state);
        match activity {
            UpdateActivity::Idle => self.set_idle_state(),
            UpdateActivity::Checking => self.set_checking_state(),
            UpdateActivity::Checked => self.set_checked_state(update_available),
            UpdateActivity::Downloading { progress } => {
                self.set_downloading_state(update_available, progress);
            }
            UpdateActivity::Downloaded => self.set_downloaded_state(update_available),
            UpdateActivity::Installing { progress: _ } => {
                self.set_installing_state(update_available);
            }
            UpdateActivity::Installed => self.set_installed_state(),
        }

        if matches!(previous_activity, UpdateActivity::Installing { .. })
            && matches!(activity, UpdateActivity::Installed)
        {
            self.present_install_finished_dialog();
        }

        *imp.last_activity.borrow_mut() = activity;
    }

    fn apply_idle_state(&self) {
        let imp = self.imp();
        self.stop_operation_progress_activity();
        self.reset_transient_update_ui();
        imp.check_button.set_visible(true);
        imp.check_button.set_sensitive(true);
        imp.download_button.set_sensitive(true);
        imp.update_button.set_sensitive(true);
        imp.download_size.set_label("Download size: unknown");
    }

    fn present_install_finished_dialog(&self) {
        let Some(window) = self.get_app_ref().and_then(|app| app.active_window()) else {
            warn!("UpdatePage: cannot present reboot dialog without an active window");
            return;
        };

        let dialog = adw::AlertDialog::builder()
            .css_name("alertdialog")
            .heading("Installation finished")
            .body("Reboot the system")
            .build();
        dialog.add_responses(&[("reboot", "Reboot now")]);
        dialog.set_default_response(Some("reboot"));
        dialog.set_close_response("reboot");

        glib::spawn_future_local(async move {
            let response = dialog.choose_future(Some(&window)).await;
            if response.as_str() == "reboot" {
                info!("UpdatePage: reboot requested after installation");
            }
        });
    }

    fn get_app_ref(&self) -> Option<ControlPanelGuiApplication> {
        gio::Application::default()
            .and_downcast::<ControlPanelGuiApplication>()
            .or_else(|| {
                self.root()
                    .and_downcast::<gtk::Window>()
                    .and_then(|window| window.application().and_downcast())
            })
    }
}

async fn fetch_system_status(app: &ControlPanelGuiApplication) -> SystemStatus {
    match app.get_sysinfo_status_from_host().await {
        Ok(status) => SystemStatus {
            ghaf_version: status.ghaf_version,
        },
        Err(e) => {
            warn!("UpdatePage: failed to fetch host version: {e}");
            SystemStatus {
                ghaf_version: String::from("unknown"),
            }
        }
    }
}

fn progress_text(value: f64) -> String {
    format!("{:.0}%", (value.clamp(0.0, 1.0) * 100.0).round())
}

#[allow(clippy::cast_precision_loss)]
fn format_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;

    if bytes >= GIB {
        let value = bytes as f64 / GIB as f64;
        format!("{value:.1} GiB")
    } else if bytes >= MIB {
        let value = bytes as f64 / MIB as f64;
        format!("{value:.1} MiB")
    } else {
        format!("{bytes} B")
    }
}

struct SystemStatus {
    ghaf_version: String,
}
