use std::cell::RefCell;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, DropDown, CheckButton, StringList};
use glib::Binding;
use glib::subclass::Signal;
use std::sync::OnceLock;
use std::process::{Command, Stdio};
use crate::settings_gobject::SettingsGObject;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/display_settings_page.ui")]
    pub struct DisplaySettingsPage {
        #[template_child]
        pub resolution_switch: TemplateChild<DropDown>,
        #[template_child]
        pub scale_switch: TemplateChild<DropDown>,
        #[template_child]
        pub light_theme_button: TemplateChild<CheckButton>,
        #[template_child]
        pub dark_theme_button: TemplateChild<CheckButton>,

        //must be read from somewhere
        pub supported_resolutions: StringList,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DisplaySettingsPage {
        const NAME: &'static str = "DisplaySettingsPage";
        type Type = super::DisplaySettingsPage;
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
    impl DisplaySettingsPage {
        #[template_callback]
        fn on_reset_clicked(&self) {
            self.obj().restore_default();
            self.obj().emit_by_name::<()>("default-display-settings", &[]);
        }
        #[template_callback]
        fn on_apply_clicked(&self) {
            let resolution_idx = self.resolution_switch.selected();
            let scale_idx = self.scale_switch.selected();
            let is_resolution_set = self.obj().set_resolution(resolution_idx);
            let is_scale_set = self.obj().set_scale(scale_idx);
            if (resolution_idx > 0 || scale_idx > 0) &&
                is_resolution_set &&
                is_scale_set {
                    self.obj().emit_by_name::<()>("display-settings-changed", &[]);
                }
        }
    }//end #[gtk::template_callbacks]

    impl ObjectImpl for DisplaySettingsPage {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            // Setup
            let obj = self.obj();
            obj.init();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("display-settings-changed")
                    .build(),
                    Signal::builder("default-display-settings")
                    .build(),
                    Signal::builder("display-settings-error")
                    .build(),
                    ]
            })
        }
    }
    impl WidgetImpl for DisplaySettingsPage {}
    impl BoxImpl for DisplaySettingsPage {}
}

glib::wrapper! {
pub struct DisplaySettingsPage(ObjectSubclass<imp::DisplaySettingsPage>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for DisplaySettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplaySettingsPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
    
    pub fn init(&self) {
        //temporary, must be read from somewhere
        let supported_resolutions = self.imp().supported_resolutions.clone();
        //the list is taken from /sys/kernel/debug/dri/0/i915_display_info
        supported_resolutions.append(&String::from("1920x1200"));
        supported_resolutions.append(&String::from("1936x1203"));
        supported_resolutions.append(&String::from("1952x1217"));
        supported_resolutions.append(&String::from("2104x1236"));
        let switch = self.imp().resolution_switch.get();
        switch.set_model(Some(&supported_resolutions));
    }

    pub fn bind(&self, _settings_object: &SettingsGObject) {
        //unbind previous ones
        self.unbind();
        //make new
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }

    pub fn restore_default(&self) {
        self.imp().resolution_switch.set_selected(0);
        self.imp().scale_switch.set_selected(0);
        self.set_resolution(0);
        self.set_scale(0);
    }

    pub fn set_resolution(&self, index: u32) -> bool {
        //default: wlr-randr --output eDP-1 --mode 1920x1200
        //custom: wlr-randr --output eDP-1 --custom-mode 'resolution@fps'
        let mut result: bool = false;
        let resolution = self.imp().supported_resolutions.string(index).unwrap();
        let output = Command::new("wlr-randr")
            .arg("--output")
            .arg("eDP-1")
            .arg(if index > 0 { "--custom-mode" } else { "--mode" })
            .arg(if index > 0 {String::from(resolution) + &String::from("@60")} else {String::from(resolution)})
            //.env("PATH", "/run/current-system/sw/bin")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("wlr-randr output: {}", stdout);

                    result = true;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("wlr-randr error: {}", stderr);
                    //emit error signal to show popup
                }
            }
            Err(e) => {
                eprintln!("Failed to execute wlr-randr: {}", e);
                //emit error signal to show popup
            }
        }
        result
    }

    pub fn set_scale(&self, index: u32) -> bool {
        let mut result: bool = false;
        let factor = match index {
            0 => 1.0,
            1 => 1.25,
            2 => 1.5,
            _ => 1.0,
        };

        let output = Command::new("wlr-randr")
            .arg("--output")
            .arg("eDP-1")
            .arg("--scale")
            .arg(&factor.to_string())
            //.env("PATH", "/run/current-system/sw/bin")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("wlr-randr scale output: {}", stdout);

                    result = true;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("wlr-randr error: {}", stderr);
                    //emit error signal
                }
            }
            Err(e) => {
                eprintln!("Failed to execute wlr-randr: {}", e);
                //emit error signal
            }
        }
        result
    }
}

