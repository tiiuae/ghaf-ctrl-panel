use std::cell::RefCell;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Label, DropDown, Scale, Image, MenuButton, Popover};
use glib::{Binding, ToValue};

use crate::vm_gobject::VMGObject;
use crate::audio_settings::AudioSettings;
use crate::security_icon::SecurityIcon;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/vm_settings.ui")]
    pub struct VMSettings {
        pub name: String,

        #[template_child]
        pub vm_name_label: TemplateChild<Label>,
        #[template_child]
        pub vm_details_label: TemplateChild<Label>,
        #[template_child]
        pub security_icon: TemplateChild<Image>,
        #[template_child]
        pub security_label: TemplateChild<Label>,
        #[template_child]
        pub audio_settings_box: TemplateChild<AudioSettings>,
        #[template_child]
        pub vm_action_menu_button: TemplateChild<MenuButton>,
        #[template_child]
        pub popover_menu: TemplateChild<Popover>,

        //current VMGObject ref
        //vm_object: &VMGObject,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VMSettings {
        const NAME: &'static str = "VMSettings";
        type Type = super::VMSettings;
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
    impl VMSettings {
        #[template_callback]
        fn on_vm_start_clicked(&self) {
            let vm_name = self.vm_name_label.label();
            println!("Start {vm_name}");
            //send message to client mod via channel in DataProvider
            //and close menu
            self.popover_menu.popdown();
        }
        #[template_callback]
        fn on_vm_shutdown_clicked(&self) {
            let vm_name = self.vm_name_label.label();
            println!("Shutdown {vm_name}");
            //send message to client mod via channel in DataProvider
            //and close menu
            self.popover_menu.popdown();
        }
        #[template_callback]
        fn on_vm_pause_clicked(&self) {
            let vm_name = self.vm_name_label.label();
            println!("Pause {vm_name}");
            //send message to client mod via channel in DataProvider
            //and close menu
            self.popover_menu.popdown();
        }
        #[template_callback]
        fn on_mic_changed(&self, value: u32) {
            println!("Mic changed: {}", value);
            //send message to client mod via channel in DataProvider
        }
        #[template_callback]
        fn on_speaker_changed(&self, value: u32) {
            println!("Speaker changed: {}", value);
            //send message to client mod via channel in DataProvider
        }
        #[template_callback]
        fn on_mic_volume_changed(&self, value: f64) {
            println!("Mic volume: {}", value);
            //send message to client mod via channel in DataProvider
        }
        #[template_callback]
        fn on_speaker_volume_changed(&self, value: f64) {
            println!("Speaker volume: {}", value);
            //send message to client mod via channel in DataProvider
        }
    }//end #[gtk::template_callbacks]

    impl ObjectImpl for VMSettings {}
    impl WidgetImpl for VMSettings {}
    impl BoxImpl for VMSettings {}
}

glib::wrapper! {
pub struct VMSettings(ObjectSubclass<imp::VMSettings>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for VMSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl VMSettings {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    /*pub fn set_data_provider(&self, data_provider: &DataProvider) {

    }

    pub fn set_current_vm_object(&self, _vm_object: &VMGObject) {
        vm_object = _vm_object;
        //bind? or set all values?
    }*/

    pub fn bind(&self, vm_object: &VMGObject) {
        //unbind previous ones
        self.unbind();
        //make new
        let name = self.imp().vm_name_label.get();
        let details = self.imp().vm_details_label.get();
        let security_icon = self.imp().security_icon.get();
        let security_label = self.imp().security_label.get();
        let mut bindings = self.imp().bindings.borrow_mut();


        let name_binding = vm_object
            .bind_property("name", &name, "label")
            //.bidirectional()
            .sync_create()
            .build();
        // Save binding
        bindings.push(name_binding);

        let details_binding = vm_object
            .bind_property("details", &details, "label")
            .sync_create()
            .build();
        // Save binding
        bindings.push(details_binding);

        let security_icon_binding = vm_object
            .bind_property("trust-level", &security_icon, "resource")
            .sync_create()
            .transform_to(move |_, value: &glib::Value| {
                let trust_level = value.get::<u8>().unwrap_or(0);
                Some(glib::Value::from(SecurityIcon::new(trust_level).0))
            })
            .build();
        // Save binding
        bindings.push(security_icon_binding);

        let security_label_binding = vm_object
            .bind_property("trust-level", &security_label, "label")
            .sync_create()
            .transform_to(move |_, value: &glib::Value| {
                let trust_level = value.get::<u8>().unwrap_or(0);
                match trust_level {//make struct like for icon?
                    0 => Some(glib::Value::from("Secure!")),
                    1 => Some(glib::Value::from("Security warning!")),
                    2 => Some(glib::Value::from("Security alert!")),
                    _ => Some(glib::Value::from("Secure!")),
                }
            })
            .build();
        // Save binding
        bindings.push(security_label_binding);
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}

