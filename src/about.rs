use gtk::glib;
use gtk::gio;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
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
            self.obj()
                .connect_notify_local(Some("root"), |obj, _| obj.refresh());
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
        self.imp()
            .ghaf_version_value
            .set_label("Ghaf Version: loading...");
        self.imp().secure_boot_value.set_label("Secure Boot: loading...");
        self.imp()
            .disk_encryption_value
            .set_label("Disk Encryption: loading...");
        self.imp()
            .yubikey_enrollment_value
            .set_label("YubiKey: loading...");
        self.imp().device_id_value.set_label("Device ID: loading...");

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
                    ) = detect_host_security_status(&app).await;
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
            self.imp()
                .ghaf_version_value
                .set_label("Ghaf Version: unknown");
            self.imp().secure_boot_value.set_label("Secure Boot: unknown");
            self.imp()
                .disk_encryption_value
                .set_label("Disk Encryption: unknown");
            self.imp()
                .yubikey_enrollment_value
                .set_label("YubiKey: unknown");
            self.imp().device_id_value.set_label("Device ID: unknown");
        }
    }

    fn get_app_ref(&self) -> Option<ControlPanelGuiApplication> {
        if let Some(app) = gio::Application::default()
            .and_then(|app| app.downcast::<ControlPanelGuiApplication>().ok())
        {
            return Some(app);
        }

        let window = self.root()?.downcast::<ControlPanelGuiWindow>().ok()?;
        window
            .application()
            .and_then(|app| app.downcast::<ControlPanelGuiApplication>().ok())
    }
}

async fn detect_host_security_status(
    app: &ControlPanelGuiApplication,
) -> (String, String, String, String, String) {
    const MAX_ATTEMPTS: usize = 6;
    const RETRY_DELAY_SECS: u32 = 1;
    let yubikey_enrollment = detect_yubikey_enrollment();
    let device_id = detect_device_id();

    for attempt in 1..=MAX_ATTEMPTS {
        match app.get_sysinfo_status_from_host().await {
            Ok(status) => {
                let ghaf_version = format!("Ghaf Version: {}", status.ghaf_version);
                let secure_boot = format_status_line("Secure Boot", &status.secure_boot);
                let disk_encryption =
                    format_status_line("Disk Encryption", &status.disk_encryption);
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
                        String::from("Ghaf Version: unknown"),
                        String::from("Secure Boot: unknown"),
                        String::from("Disk Encryption: unknown"),
                        yubikey_enrollment,
                        device_id,
                    );
                }

                glib::timeout_future_seconds(RETRY_DELAY_SECS).await;
            }
        }
    }

    (
        String::from("Ghaf Version: unknown"),
        String::from("Secure Boot: unknown"),
        String::from("Disk Encryption: unknown"),
        yubikey_enrollment,
        device_id,
    )
}

fn detect_yubikey_enrollment() -> String {
    let users_output = match Command::new("homectl").arg("list").output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            warn!(
                "AboutPage: homectl list failed with status {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
            return String::from("YubiKey: unknown");
        }
        Err(e) => {
            warn!("AboutPage: failed to run homectl list: {e}");
            return String::from("YubiKey: unknown");
        }
    };

    let users: Vec<String> = String::from_utf8_lossy(&users_output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("NAME") && !line.starts_with('-'))
        .filter_map(|line| line.split_whitespace().next().map(ToOwned::to_owned))
        .collect();

    for user in users {
        let inspect_output = match Command::new("homectl").arg("inspect").arg(&user).output() {
            Ok(output) if output.status.success() => output,
            Ok(output) => {
                warn!(
                    "AboutPage: homectl inspect {} failed with status {:?}: {}",
                    user,
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                );
                continue;
            }
            Err(e) => {
                warn!("AboutPage: failed to run homectl inspect {}: {e}", user);
                continue;
            }
        };

        let inspect_stdout = String::from_utf8_lossy(&inspect_output.stdout);
        if inspect_stdout.contains("FIDO2 Token") {
            return String::from("YubiKey: Enrolled");
        }
    }

    String::from("YubiKey: Not enrolled")
}

fn detect_device_id() -> String {
    match fs::read_to_string("/etc/common/device-id") {
        Ok(content) => {
            let value = content.trim();
            if value.is_empty() {
                String::from("Device ID: unknown")
            } else {
                format!("Device ID: {value}")
            }
        }
        Err(e) => {
            warn!("AboutPage: failed to read /persist/common/device-id: {e}");
            String::from("Device ID: unknown")
        }
    }
}

fn format_status_line(label: &str, output: &str) -> String {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some((key, value)) = trimmed.split_once(':')
            && key.trim().eq_ignore_ascii_case(label)
        {
            return format!("{label}: {}", value.trim().to_ascii_lowercase());
        }
    }

    let normalized = output.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "enabled" | "disabled" | "unknown" => format!("{label}: {normalized}"),
        _ => format!("{label}: unknown"),
    }
}
