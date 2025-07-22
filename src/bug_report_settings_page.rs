use gtk::glib;

mod imp {
    use crate::github::create_github_issue;
    use chrono::Utc;
    use glib::subclass::Signal;
    use glib::{Binding, Properties};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, Box, CheckButton, CompositeTemplate, Entry, Label, TextBuffer, TextView};
    use std::cell::RefCell;
    use std::fmt::Write;
    use std::fs;
    use std::process::{Command, Stdio};
    use std::sync::OnceLock;

    use crate::prelude::*;

    #[derive(Default)]
    pub struct Answers {
        pub issue: String,
        pub related: String,
        pub app: String,
        pub description_empty: bool,
    }

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::BugReportSettingsPage)]
    #[template(resource = "/ae/tii/ghaf/bugreport/ui/bug_report_settings_page.ui")]
    pub struct BugReportSettingsPage {
        #[template_child]
        pub qbox_3: TemplateChild<Box>,
        #[template_child]
        pub entry_1: TemplateChild<Entry>,
        #[template_child]
        pub entry_2: TemplateChild<Entry>,
        #[template_child]
        pub entry_3: TemplateChild<Entry>,
        #[template_child]
        pub title: TemplateChild<Entry>,
        #[template_child]
        pub description: TemplateChild<TextView>,
        #[template_child]
        pub description_textbuffer: TemplateChild<TextBuffer>,
        #[template_child]
        pub placeholder_textbuffer: TemplateChild<TextBuffer>,
        #[template_child]
        pub other_1: TemplateChild<CheckButton>,
        #[template_child]
        pub other_2: TemplateChild<CheckButton>,
        #[template_child]
        pub other_3: TemplateChild<CheckButton>,
        #[template_child]
        pub required_issue: TemplateChild<Label>,
        #[template_child]
        pub required_related: TemplateChild<Label>,
        #[template_child]
        pub required_app: TemplateChild<Label>,
        #[template_child]
        pub required_title: TemplateChild<Label>,
        #[template_child]
        pub required_description: TemplateChild<Label>,
        #[template_child]
        pub summary: TemplateChild<Label>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
        #[property(name = "issue", get, set, type = String, member = issue)]
        #[property(name = "related", get, set, type = String, member = related)]
        #[property(name = "app", get, set, type = String, member = app)]
        pub answers: RefCell<Answers>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BugReportSettingsPage {
        const NAME: &'static str = "BugReportSettingsPage";
        type Type = super::BugReportSettingsPage;
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
    impl BugReportSettingsPage {
        #[template_callback]
        fn entry_1_focused(&self) {
            if !self.other_1.is_active() {
                self.other_1.set_active(true);
                self.on_a1_toggled(&self.other_1);
            }
            self.obj()
                .set_property("issue", self.entry_1.text().to_string());
        }

        #[template_callback]
        fn entry_2_focused(&self) {
            if !self.other_2.is_active() {
                self.other_2.set_active(true);
                self.on_a2_toggled(&self.other_2);
            }
            self.obj()
                .set_property("related", self.entry_2.text().to_string());
        }

        #[template_callback]
        fn entry_3_focused(&self) {
            if !self.other_3.is_active() {
                self.other_3.set_active(true);
                self.on_a3_toggled(&self.other_3);
            }
            self.obj()
                .set_property("app", self.entry_3.text().to_string());
        }

        #[template_callback]
        fn on_a1_toggled(&self, button: &CheckButton) {
            if button.is_active() {
                let label = button.label();

                match label.as_deref() {
                    None => self.obj().set_property("issue", self.entry_1.text()),
                    Some(text) => self.obj().set_property("issue", text),
                }
            }
        }

        #[template_callback]
        fn on_a2_toggled(&self, button: &CheckButton) {
            if button.is_active() {
                let label = button.label();
                match label.as_deref() {
                    Some("App") => {
                        self.qbox_3.set_visible(true);
                        self.obj().set_property("related", String::from("App"));
                    } // Show the application list
                    Some(text) => {
                        self.obj().set_property("related", text.to_string());
                        self.qbox_3.set_visible(false);
                    } // Hide the application list
                    _ => {
                        self.obj()
                            .set_property("related", self.entry_2.text().to_string());
                        self.qbox_3.set_visible(false);
                    }
                }
            }
        }

        #[template_callback]
        fn on_a3_toggled(&self, button: &CheckButton) {
            if button.is_active() {
                let label = button.label();

                match label.as_deref() {
                    None => self
                        .obj()
                        .set_property("app", self.entry_3.text().to_string()),
                    Some(text) => self.obj().set_property("app", text.to_string()),
                }
            }
        }

        #[template_callback]
        fn on_description_focus_changed(&self) {
            if self.description.has_focus() {
                self.description
                    .set_buffer(Some(&self.description_textbuffer.get()));
                self.description.remove_css_class("placeholder");
            } else if self.description_textbuffer.start_iter()
                == self.description_textbuffer.end_iter()
            {
                self.description
                    .set_buffer(Some(&self.placeholder_textbuffer.get()));
                self.description.add_css_class("placeholder");
            }
        }

        #[template_callback]
        fn on_submit(&self) {
            let mut enable = true;
            let device_id_path = "/etc/common/device-id";
            let title = self.title.text().to_string();
            let this = self.obj().clone();
            let description = self.get_description_text();

            let time = Utc::now().to_string();

            let issue = self.obj().property::<String>("issue");
            let related = self.obj().property::<String>("related");
            let app = self.obj().property::<String>("app");

            let id = if let Some(id) = Self::get_device_id(device_id_path) {
                id
            } else {
                error!("Can't get device ID");
                String::new()
            };
            let sw = Self::get_sw_version().unwrap_or_else(|| {
                error!("Can't get SW Version");
                String::new()
            });

            // TODO: Get system version when it is available from host
            // let system = self.get_system_version();

            for (content, required) in [
                (&issue, &self.required_issue),
                (&related, &self.required_related),
                (&title, &self.required_title),
                (&description, &self.required_description),
            ] {
                enable &= !content.is_empty();
                required.set_visible(content.is_empty());
            }

            if app.is_empty() {
                if related == "App" {
                    enable = false;
                    self.required_app.set_visible(true);
                }
                error!("App is not selected");
            } else {
                self.required_app.set_visible(false);
            }

            if enable {
                // Prepare email content with optional attachment
                let mut email_body = format!(
                    "Time: {time}\n\n\
                                              Bug Report\n\n\
                                              Logging ID: {id}\n\
                                              SW Version: {sw}\n\
                                              Issue: {issue}\n\
                                              Relates to: {related}\n"
                );

                if !app.is_empty() {
                    let _ = write!(&mut email_body, "App: {app}\n\n");
                }

                let _ = write!(&mut email_body, "Description:\n{description}\n");

                let email_title = format!("{issue}: {title}");
                let (tx, rx) = async_channel::bounded(1);

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let _ =
                        tx.send_blocking(rt.block_on(create_github_issue(email_title, email_body)));
                });

                gtk::glib::spawn_future_local(async move {
                    let this = this.imp();

                    match rx.recv().await {
                        Ok(Ok(issue)) => {
                            error!("Issue {url} created", url = issue.url);
                            this.summary.set_label("Report sent successfully");
                            this.summary.remove_css_class("required-text");
                            this.summary.add_css_class("success-text");
                            this.summary.set_visible(true);
                        }
                        e => {
                            error!("Issue sending failed: {e:?}");
                            if let Ok(Err(e)) = e {
                                this.summary
                                    .set_label(&format!("Error when sending report{e}"));
                            } else {
                                this.summary.set_label("Error when sending report");
                            }

                            this.summary.remove_css_class("success-text");
                            this.summary.add_css_class("required-text");
                            this.summary.set_visible(true);
                        }
                    }
                });
            }
        }

        #[inline]
        fn get_device_id<P: AsRef<std::path::Path> + std::fmt::Display>(path: P) -> Option<String> {
            fs::read_to_string(&path)
                .inspect_err(|_| error!("Failed to read: {path}"))
                .ok()
        }

        #[inline]
        fn get_sw_version() -> Option<String> {
            {
                let output = Command::new("ghaf-version")
                    .env("PATH", "/run/current-system/sw/bin")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output();

                match output {
                    Ok(output) => {
                        if output.status.success() {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            debug!("ghaf-version: {stdout}");
                            Some(stdout.to_string())
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            error!("ghaf-version error: {stderr}");
                            None
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute ghaf-version: {e}");
                        None
                    }
                }
            }
        }

        // TODO: This will be used when the system version is readible from VM
        #[allow(dead_code)]
        #[inline]
        fn _get_system_version() -> String {
            let manufacturer: String;
            let version: String;
            let product: String;
            let sku: String;

            let output = Command::new("dmidecode")
                .arg("-s")
                .arg("system-manufacturer")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        debug!("system-manufacturer: {stdout}");
                        manufacturer = stdout.to_string();
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        error!("system-manufacturer: {stderr}");
                        manufacturer = String::from("no-manufacturer");
                    }
                }
                Err(e) => {
                    error!("Failed to execute dmidecode: {e}");
                    manufacturer = String::from("manufacturer-failed");
                }
            }

            let output = Command::new("dmidecode")
                .arg("-s")
                .arg("system-version")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        debug!("system-version: {stdout}");
                        version = stdout.to_string();
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        error!("system-version: {stderr}");
                        version = String::from("no-version");
                    }
                }
                Err(e) => {
                    error!("Failed to execute dmidecode: {e}");
                    version = String::from("version-failed");
                }
            }

            let output = Command::new("dmidecode")
                .arg("-s")
                .arg("system-product-name")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        debug!("product: {stdout}");
                        product = stdout.to_string();
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        error!("product: {stderr}");
                        product = String::from("no-product");
                    }
                }
                Err(e) => {
                    error!("Failed to execute dmidecode: {e}");
                    product = String::from("product-failed");
                }
            }

            let output = Command::new("dmidecode")
                .arg("-s")
                .arg("system-sku-number")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        debug!("sku-number: {stdout}");
                        sku = stdout.to_string();
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        error!("sku-number: {stderr}");
                        sku = String::from("no-sku-number");
                    }
                }
                Err(e) => {
                    error!("Failed to execute dmidecode: {e}");
                    sku = String::from("sku-number-failed");
                }
            }

            format!("{manufacturer} {version} {product} {sku}")
        }

        #[inline]
        fn get_description_text(&self) -> String {
            self.description_textbuffer
                .text(
                    &self.description_textbuffer.start_iter(),
                    &self.description_textbuffer.end_iter(),
                    false,
                )
                .to_string()
        }
    } //end #[gtk::template_callbacks]

    #[glib::derived_properties]
    impl ObjectImpl for BugReportSettingsPage {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();
        }
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("toggled").build(),
                    Signal::builder("clicked").build(),
                ]
            })
        }
    }
    impl WidgetImpl for BugReportSettingsPage {}
    impl BoxImpl for BugReportSettingsPage {}
}

glib::wrapper! {
pub struct BugReportSettingsPage(ObjectSubclass<imp::BugReportSettingsPage>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for BugReportSettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl BugReportSettingsPage {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("issue", "")
            .property("related", "")
            .property("app", "")
            .build()
    }
}
