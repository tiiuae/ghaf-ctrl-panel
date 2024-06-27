use std::cell::RefCell;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, DropDown, Scale};
use glib::Binding;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/audio_settings.ui")]
    pub struct AudioSettings {
        pub name: String,

        #[template_child]
        pub mic_switch: TemplateChild<DropDown>,
        #[template_child]
        pub speaker_switch: TemplateChild<DropDown>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AudioSettings {
        const NAME: &'static str = "AudioSettings";
        type Type = super::AudioSettings;
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
    impl AudioSettings {
        #[template_callback]
        fn on_mic_changed(&self) {
            println!("Mic changed!");
        }
        #[template_callback]
        fn on_speaker_changed(&self) {
            println!("Speaker changed!");
        }
        #[template_callback]
        fn on_mic_volume_changed(scale: &Scale) {
            let value = scale.value();
            println!("mic volume: {value}");
        }
        #[template_callback]
        fn on_speaker_volume_changed(scale: &Scale) {
            let value = scale.value();
            println!("Speaker volume: {value}");
        }
    }//end #[gtk::template_callbacks]

    impl ObjectImpl for AudioSettings {}
    impl WidgetImpl for AudioSettings {}
    impl BoxImpl for AudioSettings {}
}

glib::wrapper! {
pub struct AudioSettings(ObjectSubclass<imp::AudioSettings>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioSettings {
    pub fn new() -> Self {
        //glib::Object::new::<Self>()
        glib::Object::builder().build()
    }

    /*
    pub fn bind(&self, vm_object: &VMGObject) {
        let title = self.imp().title_label.get();
        let subtitle = self.imp().subtitle_label.get();
        let mut bindings = self.imp().bindings.borrow_mut();


        let title_binding = vm_object
            .bind_property("name", &title, "label")
            //.bidirectional()
            .sync_create()
            .build();
        // Save binding
        bindings.push(title_binding);

        let subtitle_binding = vm_object
            .bind_property("details", &subtitle, "label")
            .sync_create()
            .build();
        // Save binding
        bindings.push(subtitle_binding);

        // Bind `task_object.completed` to `task_row.content_label.attributes`
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
    }
    // ANCHOR_END: bind

    // ANCHOR: unbind
    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
    */
}

