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
            println!("Reset to defaults!");
            self.obj().set_resolution(0);
            self.obj().emit_by_name::<()>("default-display-settings", &[]);
        }
        #[template_callback]
        fn on_apply_clicked(&self) {
            let resolution_idx = self.resolution_switch.selected();
            let scale_idx = self.scale_switch.selected();
            self.obj().set_resolution(resolution_idx);
            self.obj().set_scale(scale_idx);
            //to test popup appearance
            //self.obj().emit_by_name::<()>("resolution-changed", &[&value]);
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
                    Signal::builder("resolution-changed")
                    .param_types([u32::static_type()])
                    .build(),
                    Signal::builder("default-display-settings")
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

    pub fn set_resolution(&self, index: u32) {
        //default: wlr-randr --output eDP-1 --mode 1920x1200
        //custom: wlr-randr --output eDP-1 --custom-mode 'resolution@fps'
        let resolution = self.imp().supported_resolutions.string(index).unwrap();
        let command = if index > 0 {
            String::from("wlr-randr --output eDP-1 --custom-mode") + &resolution + &String::from("@60")
        } else {
            String::from("wlr-randr --output eDP-1 --mode ") + &resolution
        };
        let output = Command::new(command.as_str())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("wlr-randr output: {}", stdout);

                    if index > 0 {
                        self.emit_by_name::<()>("resolution-changed", &[&index]);
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("wlr-randr error: {}", stderr);
                }
            }
            Err(e) => {
                eprintln!("Failed to execute wlr-randr: {}", e);
            }
        }
    }

    pub fn set_scale(&self, index: u32) {
        let factor = match index {
            0 => 1.0,
            1 => 1.25,
            2 => 1.5,
            _ => 1.0,
        };

        let command = String::from("wlr-randr --output eDP-1 --scale ") + &factor.to_string();

        let output = Command::new(command.as_str())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("wlr-randr scale output: {}", stdout);

                    if index > 0 {
                        self.emit_by_name::<()>("resolution-changed", &[&index]);
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("wlr-randr error: {}", stderr);
                }
            }
            Err(e) => {
                eprintln!("Failed to execute wlr-randr: {}", e);
            }
        }
    }
}

