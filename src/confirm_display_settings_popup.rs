use std::cell::RefCell;
use std::sync::OnceLock;
use std::rc::Rc;
use std::time::Duration;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate, Button, Label};
use glib::Binding;
use glib::subclass::Signal;
use glib::timeout_add_local;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/confirm_display_settings_popup.ui")]
    pub struct ConfirmDisplaySettingsPopup {
        #[template_child]
        pub countdown_label: TemplateChild<Label>,
        #[template_child]
        pub ok_button: TemplateChild<Button>,
        #[template_child]
        pub reset_button: TemplateChild<Button>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConfirmDisplaySettingsPopup {
        const NAME: &'static str = "ConfirmDisplaySettingsPopup";
        type Type = super::ConfirmDisplaySettingsPopup;
        type ParentType = gtk::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl ConfirmDisplaySettingsPopup {
        #[template_callback]
        fn on_ok_clicked(&self) {
            self.obj().close();
        }
        #[template_callback]
        fn on_reset_clicked(&self) {
            self.obj().emit_by_name::<()>("reset-default", &[]);
            self.obj().close();
        }
    }//end #[gtk::template_callbacks]

    impl ObjectImpl for ConfirmDisplaySettingsPopup {
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
                    Signal::builder("reset-default")
                    .build()
                    ]
            })
        }
    }
    impl WidgetImpl for ConfirmDisplaySettingsPopup {}
    impl BoxImpl for ConfirmDisplaySettingsPopup {}
    impl WindowImpl for ConfirmDisplaySettingsPopup {}
}

glib::wrapper! {
pub struct ConfirmDisplaySettingsPopup(ObjectSubclass<imp::ConfirmDisplaySettingsPopup>)
@extends gtk::Widget, gtk::Window, @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for ConfirmDisplaySettingsPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfirmDisplaySettingsPopup {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn init(&self) {}

    pub fn launch_close_timer(&self, sec: u32) {
        let label = self.imp().countdown_label.get();
        label.set_text(&format!("Closing in: {} seconds", sec));

        let countdown = Rc::new(RefCell::new(sec));

        let popup = self.clone();
        let label_clone = label.clone();
        let countdown_clone = countdown.clone();
        timeout_add_local(Duration::from_secs(1), move || {
            let mut remaining_time = countdown_clone.borrow_mut();
            *remaining_time -= 1;

            label_clone.set_text(&format!("Closing in: {} seconds", *remaining_time));

            if *remaining_time == 0 {
                popup.close();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }
}