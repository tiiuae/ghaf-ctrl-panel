use std::cell::RefCell;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Label, DropDown, Scale};
use glib::Binding;

use crate::vm_gobject::VMGObject;
use crate::audio_settings::AudioSettings;

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
        pub vm_action_menu_button: TemplateChild<DropDown>,
        #[template_child]
        pub audio_settings_box: TemplateChild<AudioSettings>,

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
        fn on_vm_action_selected(&self) {
            println!("Action changed!");
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

        //block was left here as example!
        /*/ Bind `task_object.completed` to `task_row.content_label.attributes`
        let content_label_binding = task_object
            .bind_property("completed", &content_label, "attributes")
            .sync_create()
            .transform_to(|_, active| {
                let attribute_list = AttrList::new();
                if active {
                    // If "active" is true, content of the label will be strikethrough
                    let attribute = AttrInt::new_strikethrough(true);
                    attribute_list.insert(attribute);
                }
                Some(attribute_list.to_value())
            })
            .build();
        // Save binding
        bindings.push(content_label_binding);
        */
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}

