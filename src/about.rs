use gtk::glib;
use gtk::gio;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use serde_json::Value;
use std::fs;
use std::process::Command;

use crate::application::ControlPanelGuiApplication;
use crate::prelude::*;
use crate::window::ControlPanelGuiWindow;

mod imp {
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{CompositeTemplate, Label, gio, glib};
    use std::cell::RefCell;

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
    #[template(resource = "/ae/tii/ghaf/controlpanelgui/ui/about.ui")]
    pub struct AboutPage {
        #[template_child]
        pub ghaf_version: TemplateChild<Label>,
        #[template_child]
        pub secure_boot: TemplateChild<Label>,
        #[template_child]
        pub disk_encryption: TemplateChild<Label>,
        #[template_child]
        pub yubikey_enrollment: TemplateChild<Label>,
        #[template_child]
        pub device_id: TemplateChild<Label>,
        pub(super) refresh_cancel: RefCell<Option<CancelGuard>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AboutPage {
        const NAME: &'static str = "AboutPage";
        type Type = super::AboutPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AboutPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().connect_root_notify(super::AboutPage::refresh);
            self.obj().refresh();
        }

        fn dispose(&self) {
            self.refresh_cancel.borrow_mut().take();
            self.dispose_template();
        }
    }

    impl WidgetImpl for AboutPage {}
    impl BoxImpl for AboutPage {}
}

glib::wrapper! {
pub struct AboutPage(ObjectSubclass<imp::AboutPage>)
    @extends gtk::Widget, gtk::Box,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for AboutPage {
    fn default() -> Self {
        Self::new()
    }
}

struct SystemStatus {
    ghaf_version: String,
    secure_boot: String,
    disk_encryption: String,
    yubikey_enrollment: String,
    device_id: String,
}

impl AboutPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn refresh(&self) {
        debug!("AboutPage: refresh security status");
        self.imp().ghaf_version.set_label("loading...");
        self.imp()
            .secure_boot
            .set_markup(&format_status_loading_markup());
        self.imp()
            .disk_encryption
            .set_markup(&format_status_loading_markup());
        self.imp().yubikey_enrollment.set_label("loading...");
        self.imp().device_id.set_label("loading...");

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
                        page.imp().ghaf_version.set_label(&status.ghaf_version);
                        page.imp().secure_boot.set_markup(&status.secure_boot);
                        page.imp()
                            .disk_encryption
                            .set_markup(&status.disk_encryption);
                        page.imp()
                            .yubikey_enrollment
                            .set_label(&status.yubikey_enrollment);
                        page.imp().device_id.set_label(&status.device_id);
                    }
                ),
                cancellable,
            ));
        } else {
            warn!("AboutPage: no app ref, cannot query host security status");
            self.imp().ghaf_version.set_label("unknown");
            self.imp()
                .secure_boot
                .set_markup(&format_optional_bool_status(None));
            self.imp()
                .disk_encryption
                .set_markup(&format_optional_bool_status(None));
            self.imp().yubikey_enrollment.set_label("unknown");
            self.imp().device_id.set_label("unknown");
        }
    }

    fn get_app_ref(&self) -> Option<ControlPanelGuiApplication> {
        if let Some(app) = gio::Application::default()
            .and_then(|app| app.downcast::<ControlPanelGuiApplication>().ok())
        {
            return Some(app);
        }

        self.root()
            .and_downcast::<ControlPanelGuiWindow>()
            .and_then(|window| window.application())
            .and_downcast::<ControlPanelGuiApplication>()
    }
}

async fn fetch_system_status(
    app: &ControlPanelGuiApplication,
) -> SystemStatus {
    const MAX_ATTEMPTS: usize = 6;
    const RETRY_DELAY_SECS: u32 = 1;
    let yubikey_enrollment = fetch_yubikey_enrollment().await;
    let device_id = detect_device_id();

    for attempt in 1..=MAX_ATTEMPTS {
        match app.get_sysinfo_status_from_host().await {
            Ok(status) => {
                let ghaf_version = normalize_status_value(&status.ghaf_version);
                let secure_boot = format_optional_bool_status(status.secure_boot);
                let disk_encryption = format_optional_bool_status(status.disk_encryption);
                debug!(
                    "AboutPage: host sysinfo status via ghaf-host (attempt {attempt}/{MAX_ATTEMPTS}): ghaf_version={ghaf_version}, secure_boot={secure_boot}, disk_encryption={disk_encryption}, yubikey={yubikey_enrollment}, device_id={device_id}"
                );
                return SystemStatus {
                    ghaf_version,
                    secure_boot: secure_boot.to_string(),
                    disk_encryption: disk_encryption.to_string(),
                    yubikey_enrollment,
                    device_id,
                };
            }
            Err(e) => {
                let message = e.to_string();
                let is_not_connected = message.contains("Not connected");
                warn!(
                    "AboutPage: ghaf-host sysinfo query failed (attempt {attempt}/{MAX_ATTEMPTS}): {message}"
                );

                if !is_not_connected || attempt == MAX_ATTEMPTS {
                    break;
                }

                glib::timeout_future_seconds(RETRY_DELAY_SECS).await;
            }
        }
    }

    SystemStatus {
        ghaf_version: String::from("unknown"),
        secure_boot: String::from("unknown"),
        disk_encryption: String::from("unknown"),
        yubikey_enrollment,
        device_id,
    }
}

async fn fetch_yubikey_enrollment() -> String {
    let (tx, rx) = async_channel::bounded(1);
    let worker = std::thread::spawn(move || {
        let _ = tx.send_blocking(detect_yubikey_enrollment_blocking());
    });
    let result = match rx.recv().await {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => {
            warn!("AboutPage: failed to detect YubiKey enrollment: {e}");
            String::from("unknown")
        }
        Err(e) => {
            warn!("AboutPage: failed to receive YubiKey enrollment status: {e}");
            String::from("unknown")
        }
    };
    let _ = worker.join();
    result
}

fn detect_yubikey_enrollment_blocking() -> Result<String, anyhow::Error> {
    let users_output = Command::new("homectl")
        .args(["list", "-j"])
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run `homectl list -j`: {e}"))?;

    if !users_output.status.success() {
        anyhow::bail!(
            "`homectl list -j` failed with status {:?}: {}",
            users_output.status.code(),
            String::from_utf8_lossy(&users_output.stderr)
        );
    }

    let users_json: Value = serde_json::from_slice(&users_output.stdout)
        .map_err(|e| anyhow::anyhow!("failed to parse `homectl list -j` output: {e}"))?;
    let users = extract_homectl_usernames(&users_json);
    if users.is_empty() {
        return Ok(String::from("Not enrolled"));
    }

    let inspect_output = Command::new("homectl")
        .args(["inspect", "-j"])
        .args(&users)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run `homectl inspect -j` for users {:?}: {e}", users))?;

    if !inspect_output.status.success() {
        anyhow::bail!(
            "`homectl inspect -j` failed for users {:?} with status {:?}: {}",
            users,
            inspect_output.status.code(),
            String::from_utf8_lossy(&inspect_output.stderr)
        );
    }

    let inspect_output = String::from_utf8_lossy(&inspect_output.stdout);
    let inspect_records: Vec<Value> = inspect_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str::<Value>)
        .collect::<Result<_, _>>()
        .map_err(|e| {
            anyhow::anyhow!(
                "failed to parse `homectl inspect -j` output for users {:?}: {e}",
                users
            )
        })?;

    // `homectl inspect -j` returns one JSON object per line. Report enrolled if any
    // listed user has a FIDO2 credential.
    if inspect_records.iter().any(has_fido2_hmac_credential) {
        return Ok(String::from("Enrolled"));
    };

    Ok(String::from("Not enrolled"))
}

fn extract_homectl_usernames(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flat_map(|items| items.iter())
        .filter_map(|item| item.get("name").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

fn has_fido2_hmac_credential(value: &Value) -> bool {
    match value {
        Value::Object(obj) => {
            if let Some(field) = obj.get("fido2HmacCredential") {
                return field.as_array().is_some_and(|credentials| !credentials.is_empty());
            }
            obj.values().any(has_fido2_hmac_credential)
        }
        Value::Array(items) => items.iter().any(has_fido2_hmac_credential),
        _ => false,
    }
}

fn detect_device_id() -> String {
    match fs::read_to_string("/etc/common/device-id") {
        Ok(content) => {
            let value = content.trim();
            if value.is_empty() {
                String::from("unknown")
            } else {
                value.to_owned()
            }
        }
        Err(e) => {
            warn!("AboutPage: failed to read /etc/common/device-id: {e}");
            String::from("unknown")
        }
    }
}

fn normalize_status_value(output: &str) -> String {
    let value = output.trim();
    if value.is_empty() {
        return String::from("unknown");
    }
    value.to_owned()
}

fn format_status_loading_markup() -> &'static str {
    "<span weight=\"600\">loading...</span>"
}

fn format_optional_bool_status(status: Option<bool>) -> &'static str {
    match status {
        Some(true) => "<span weight=\"700\" foreground=\"#2f9e44\">Enabled</span>",
        Some(false) => "<span weight=\"700\" foreground=\"#ff0000\">Disabled</span>",
        None => "<span weight=\"600\" foreground=\"#6b7280\">Unknown</span>",
    }
}
