use std::cell::RefCell;
use std::sync::OnceLock;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Box, DropDown, Scale};
use glib::{Binding, Properties};
use glib::subclass::Signal;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::AudioSettings)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/audio_settings.ui")]
    pub struct AudioSettings {
        pub name: String,

        #[template_child]
        pub mic_switch: TemplateChild<DropDown>,
        #[template_child]
        pub mic_volume: TemplateChild<Scale>,
        #[template_child]
        pub speaker_switch: TemplateChild<DropDown>,
        #[template_child]
        pub speaker_volume: TemplateChild<Scale>,
        #[template_child]
        pub footer: TemplateChild<Box>,

        #[property(name = "footer-visible", get, set, type = bool)]
        footer_visible: RefCell<bool>,

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
            let value = self.mic_switch.selected();
            //or selected_item() to get object and cast to string
            println!("Mic changed! {}", value);
            self.obj().emit_by_name::<()>("mic-changed", &[&value]);
        }
        #[template_callback]
        fn on_speaker_changed(&self) {
            let value = self.speaker_switch.selected();
            println!("Speaker changed! {}", value);
            self.obj().emit_by_name::<()>("speaker-changed", &[&value]);
        }
        #[template_callback]
        fn on_mic_volume_changed(&self, scale: &Scale) {
            let value = scale.value();
            self.obj().emit_by_name::<()>("mic-volume-changed", &[&value]);
        }
        #[template_callback]
        fn on_speaker_volume_changed(&self, scale: &Scale) {
            let value = scale.value();
            self.obj().emit_by_name::<()>("speaker-volume-changed", &[&value]);
        }
        #[template_callback]
        fn on_reset_clicked(&self) {
            println!("Reset to defaults!");
            self.obj().emit_by_name::<()>("set-defaults", &[]);
        }
        #[template_callback]
        fn on_save_clicked(&self) {
            println!("Apply new!");
            let mic = self.mic_switch.selected();
            let speaker = self.speaker_switch.selected();
            let mic_volume = self.mic_volume.value();
            let speaker_volume = self.speaker_volume.value();
            self.obj().emit_by_name::<()>("apply-new", &[&mic, &speaker, &mic_volume, &speaker_volume]);
        }
    }//end #[gtk::template_callbacks]

    #[glib::derived_properties]
    impl ObjectImpl for AudioSettings {
        fn constructed(&self) {
            self.parent_constructed();
    
            // After the object is constructed, bind the footer visibilty property
            let obj = self.obj();
            obj.bind_property("footer-visible", &self.footer.get(), "visible")
                .flags(glib::BindingFlags::DEFAULT)
                .build();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("mic-changed")
                    .param_types([u32::static_type()])
                    .build(),
                    Signal::builder("speaker-changed")
                    .param_types([u32::static_type()])
                    .build(),
                    Signal::builder("mic-volume-changed")
                    .param_types([f64::static_type()])
                    .build(),
                    Signal::builder("speaker-volume-changed")
                    .param_types([f64::static_type()])
                    .build(),
                    Signal::builder("set-defaults")
                    .build(),
                    Signal::builder("apply-new")
                    .param_types([u32::static_type(), u32::static_type(), f64::static_type(), f64::static_type()])
                    .build(),
                    ]
            })
        }
    }

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

