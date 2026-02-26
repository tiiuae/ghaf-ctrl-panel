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
    use gtk::{CompositeTemplate, Label, glib};

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/ae/tii/ghaf/controlpanelgui/ui/about.ui")]
    pub struct AboutPage {
        #[template_child]
        pub ghaf_version_value: TemplateChild<Label>,
        #[template_child]
        pub secure_boot_value: TemplateChild<Label>,
        #[template_child]
        pub disk_encryption_value: TemplateChild<Label>,
        #[template_child]
        pub yubikey_enrollment_value: TemplateChild<Label>,
        #[template_child]
        pub device_id_value: TemplateChild<Label>,
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

impl AboutPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn refresh(&self) {
        debug!("AboutPage: refresh security status");
        self.imp().ghaf_version_value.set_label("loading...");
        self.imp().secure_boot_value.set_label("loading...");
        self.imp().disk_encryption_value.set_label("loading...");
        self.imp().yubikey_enrollment_value.set_label("loading...");
        self.imp().device_id_value.set_label("loading...");

        if let Some(app) = self.get_app_ref() {
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = page)]
                self,
                async move {
                    let (
                        ghaf_version,
                        secure_boot_status,
                        disk_encryption_status,
                        yubikey_enrollment_status,
                        device_id_status,
                    ) = fetch_system_status(&app).await;
                    page.imp().ghaf_version_value.set_label(&ghaf_version);
                    page.imp().secure_boot_value.set_label(&secure_boot_status);
                    page.imp()
                        .disk_encryption_value
                        .set_label(&disk_encryption_status);
                    page.imp()
                        .yubikey_enrollment_value
                        .set_label(&yubikey_enrollment_status);
                    page.imp().device_id_value.set_label(&device_id_status);
                }
            ));
        } else {
            warn!("AboutPage: no app ref, cannot query host security status");
            self.imp().ghaf_version_value.set_label("unknown");
            self.imp().secure_boot_value.set_label("unknown");
            self.imp().disk_encryption_value.set_label("unknown");
            self.imp().yubikey_enrollment_value.set_label("unknown");
            self.imp().device_id_value.set_label("unknown");
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
) -> (String, String, String, String, String) {
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
                return (
                    ghaf_version,
                    secure_boot,
                    disk_encryption,
                    yubikey_enrollment,
                    device_id,
                );
            }
            Err(e) => {
                let message = e.to_string();
                let is_not_connected = message.contains("Not connected");
                warn!(
                    "AboutPage: ghaf-host sysinfo query failed (attempt {attempt}/{MAX_ATTEMPTS}): {message}"
                );

                if !is_not_connected || attempt == MAX_ATTEMPTS {
                    return (
                        String::from("unknown"),
                        String::from("unknown"),
                        String::from("unknown"),
                        yubikey_enrollment,
                        device_id,
                    );
                }

                glib::timeout_future_seconds(RETRY_DELAY_SECS).await;
            }
        }
    }

    (
        String::from("unknown"),
        String::from("unknown"),
        String::from("unknown"),
        yubikey_enrollment,
        device_id,
    )
}

async fn fetch_yubikey_enrollment() -> String {
    let (tx, rx) = async_channel::bounded(1);
    let worker = std::thread::spawn(move || {
        let _ = tx.send_blocking(detect_yubikey_enrollment_blocking());
    });
    let result = rx.recv().await.unwrap_or_else(|_| String::from("unknown"));
    let _ = worker.join();
    result
}

fn detect_yubikey_enrollment_blocking() -> String {
    let users_output = match Command::new("homectl").args(["list", "-j"]).output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            warn!(
                "AboutPage: homectl list -j failed with status {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
            return String::from("unknown");
        }
        Err(e) => {
            warn!("AboutPage: failed to run homectl list -j: {e}");
            return String::from("unknown");
        }
    };

    let users_json: Value = match serde_json::from_slice(&users_output.stdout) {
        Ok(v) => v,
        Err(e) => {
            warn!("AboutPage: failed to parse homectl list -j output: {e}");
            return String::from("unknown");
        }
    };
    let users = extract_homectl_usernames(&users_json);
    if users.is_empty() {
        return String::from("Not enrolled");
    }

    let inspect_output = match Command::new("homectl")
        .args(["inspect", "-j"])
        .args(&users)
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            warn!(
                "AboutPage: homectl inspect -j failed for users {:?} with status {:?}: {}",
                users,
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
            return String::from("unknown");
        }
        Err(e) => {
            warn!("AboutPage: failed to run homectl inspect -j for users {:?}: {e}", users);
            return String::from("unknown");
        }
    };

    let inspect_json: Value = match serde_json::from_slice(&inspect_output.stdout) {
        Ok(v) => v,
        Err(e) => {
            warn!(
                "AboutPage: failed to parse homectl inspect -j output for users {:?}: {e}",
                users
            );
            return String::from("unknown");
        }
    };
    // Intended behavior: report enrolled if any listed user has a FIDO2 credential.
    if has_fido2_hmac_credential(&inspect_json) {
        return String::from("Enrolled");
    }

    String::from("Not enrolled")
}

fn extract_homectl_usernames(value: &Value) -> Vec<String> {
    fn username_from_object(obj: &serde_json::Map<String, Value>) -> Option<String> {
        ["userName", "username", "name", "user", "Name", "UserName"]
            .iter()
            .find_map(|k| obj.get(*k).and_then(Value::as_str))
            .map(ToOwned::to_owned)
    }

    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.as_object().and_then(username_from_object))
            .collect(),
        Value::Object(obj) => obj
            .get("homes")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_object().and_then(username_from_object))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn has_fido2_hmac_credential(value: &Value) -> bool {
    match value {
        Value::Object(obj) => {
            if let Some(field) = obj.get("fido2HmacCredential") {
                return !field.is_null()
                    && match field {
                        Value::Array(a) => !a.is_empty(),
                        Value::String(s) => !s.trim().is_empty(),
                        _ => true,
                    };
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

fn format_optional_bool_status(status: Option<bool>) -> String {
    match status {
        Some(true) => String::from("Enabled"),
        Some(false) => String::from("Disabled"),
        None => String::from("Unknown"),
    }
}
