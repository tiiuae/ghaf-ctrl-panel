use gio::ListModel;
use glib::subclass::Signal;
use glib::{Binding, Object, Properties, Variant};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{
    gio, glib, Box, CompositeTemplate, DropDown, FilterListModel, Label, ListItem, Scale,
    SignalListItemFactory, SingleSelection,
};
use imp::AudioDeviceUserType;
use std::cell::RefCell;
use std::sync::OnceLock;

use crate::audio_device_gobject::imp::AudioDeviceType;
use crate::audio_device_gobject::AudioDeviceGObject;
use crate::typed_list_store::imp::TypedCustomFilter;

mod imp {
    use super::*;

    #[derive(PartialEq)]
    pub enum AudioDeviceUserType {
        Mic = 0,
        Speaker = 1,
    }

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

        pub speaker_bindings: RefCell<Vec<Binding>>,
        pub mic_bindings: RefCell<Vec<Binding>>,
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
        fn on_advanced_settings_clicked(&self) {
            self.obj()
                .emit_by_name::<()>("open-advanced-audio-settings", &[]);
        }
        #[template_callback]
        fn on_mic_changed(&self) {
            if let Some(selected_item) = self.mic_switch.selected_item() {
                if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                    let vec = vec![obj.id(), obj.dev_type()];
                    let variant = vec.to_variant();
                    self.obj().emit_by_name::<()>("mic-changed", &[&variant]);

                    // binding
                    self.obj().bind_mic_volume_property(obj);
                }
            }
        }
        #[template_callback]
        fn on_speaker_changed(&self) {
            if let Some(selected_item) = self.speaker_switch.selected_item() {
                if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                    let vec = vec![obj.id(), obj.dev_type()];
                    let variant = vec.to_variant();
                    self.obj()
                        .emit_by_name::<()>("speaker-changed", &[&variant]);

                    // binding
                    self.obj().bind_speaker_volume_property(obj);
                }
            }
        }
        #[template_callback]
        fn on_mic_volume_changed(&self, scale: &Scale) {
            if let Some(selected_item) = self.mic_switch.selected_item() {
                if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                    let value = scale.value() as i32;
                    if (value != obj.volume()) {
                        let id = obj.id();
                        let dev_type = obj.dev_type();
                        let vec = vec![id, dev_type, value];
                        let variant = vec.to_variant();
                        self.obj()
                            .emit_by_name::<()>("mic-volume-changed", &[&variant]);
                        //save new value
                        obj.set_volume(value);
                    }
                }
            }
        }
        #[template_callback]
        fn on_speaker_volume_changed(&self, scale: &Scale) {
            if let Some(selected_item) = self.speaker_switch.selected_item() {
                if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                    let value = scale.value() as i32;
                    if (value != obj.volume()) {
                        let id = obj.id();
                        let dev_type = obj.dev_type();
                        let vec = vec![id, dev_type, value];
                        let variant = vec.to_variant();
                        self.obj()
                            .emit_by_name::<()>("speaker-volume-changed", &[&variant]);
                        //save new value
                        obj.set_volume(value);
                    }
                }
            }
        }
        #[template_callback]
        fn on_reset_clicked(&self) {
            println!("Reset to defaults!");
            self.obj().emit_by_name::<()>("set-defaults", &[]);
        }
        #[template_callback]
        fn on_save_clicked(&self) {
            println!("Save new audio settings");
            let mic = self.mic_switch.selected();
            let speaker = self.speaker_switch.selected();
            let mic_volume = self.mic_volume.value();
            let speaker_volume = self.speaker_volume.value();
            self.obj()
                .emit_by_name::<()>("save-new", &[&mic, &speaker, &mic_volume, &speaker_volume]);
        }
    } //end #[gtk::template_callbacks]

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
                        .param_types([Variant::static_type()])
                        .build(),
                    Signal::builder("speaker-changed")
                        .param_types([Variant::static_type()])
                        .build(),
                    Signal::builder("mic-volume-changed")
                        .param_types([Variant::static_type()])
                        .build(),
                    Signal::builder("speaker-volume-changed")
                        .param_types([Variant::static_type()])
                        .build(),
                    Signal::builder("set-defaults").build(),
                    Signal::builder("save-new")
                        .param_types([
                            u32::static_type(),
                            u32::static_type(),
                            f64::static_type(),
                            f64::static_type(),
                        ])
                        .build(),
                    Signal::builder("open-advanced-audio-settings").build(),
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

    pub fn set_audio_devices(&self, devices: ListModel) {
        //setup factory
        self.setup_factory(AudioDeviceUserType::Mic);
        self.setup_factory(AudioDeviceUserType::Speaker);

        //Create filter: outputs
        let outputs_filter = TypedCustomFilter::new(|obj: &AudioDeviceGObject| {
            obj.dev_type() == AudioDeviceType::Sink as i32 //only one for now
        });

        //Create filter: inputs
        let inputs_filter = TypedCustomFilter::new(|obj: &AudioDeviceGObject| {
            obj.dev_type() == AudioDeviceType::Source as i32 //only one for now
        });

        let count = devices.n_items();
        println!("Devices came to audio settings: {count}");

        //setup model for outputs
        let output_filter_model = FilterListModel::new(Some(devices.clone()), Some(outputs_filter));
        let output_model = SingleSelection::new(Some(output_filter_model));
        output_model.set_autoselect(false); //to select only on click, not on hover

        //setup model for inputs
        let input_filter_model = FilterListModel::new(Some(devices), Some(inputs_filter));
        let input_model = SingleSelection::new(Some(input_filter_model));
        input_model.set_autoselect(false); //to select only on click, not on hover

        /*/selection changed signal doesn't work automatically for some reason
        output_model.connect_selection_changed(
            glib::clone!(@strong self as widget => move |selection_model, _, _| {
                if let Some(selected_item) = selection_model.selected_item() {
                    println!("Selected: {}", selection_model.selected());
                    if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                        bind_speaker_volume_property(obj);
                    }
                } else {
                    println!("No item selected");
                }
            }),
        );*/
        output_model.connect_items_changed(
            glib::clone!(@strong self as widget => move |selection_model, position, removed, added| {
                println!("Output model: Items changed at position {}, removed: {}, added: {}", position, removed, added);
                if let Some(selected_item) = selection_model.selected_item() {
                    if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                        widget.bind_speaker_volume_property(obj);
                    }
                } else {
                    println!("No item selected");
                }
            })
        );
        /*input_model.connect_selection_changed(
            glib::clone!(@strong self as widget => move |selection_model, _, _| {
                if let Some(selected_item) = selection_model.selected_item() {
                    println!("Selected: {}", selection_model.selected());
                    if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                        bind_mic_volume_property(obj);
                    }
                } else {
                    println!("No item selected");
                }
            }),
        );*/
        input_model.connect_items_changed(
            glib::clone!(@strong self as widget => move |selection_model, position, removed, added| {
                println!("Input model: Items changed at position {}, removed: {}, added: {}", position, removed, added);
                if let Some(selected_item) = selection_model.selected_item() {
                    if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                        widget.bind_mic_volume_property(obj);
                    }
                } else {
                    println!("No item selected");
                }
            })
        );

        self.imp().speaker_switch.set_model(Some(&output_model));
        self.imp().mic_switch.set_model(Some(&input_model));

        //initial selection and binding
        if (input_model.n_items() > 0) {
            input_model.selection_changed(0u32, input_model.n_items());
            if let Some(selected_item) = input_model.selected_item() {
                if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                    self.bind_mic_volume_property(obj);
                }
            }
        }

        if (output_model.n_items() > 0) {
            output_model.selection_changed(0u32, output_model.n_items());
            if let Some(selected_item) = output_model.selected_item() {
                if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                    self.bind_speaker_volume_property(obj);
                }
            }
        }
    }

    pub fn bind_speaker_volume_property(&self, obj: &AudioDeviceGObject) {
        let mut bindings = self.imp().speaker_bindings.borrow_mut();
        let adjustment = self.imp().speaker_volume.get().adjustment();

        for binding in bindings.drain(..) {
            binding.unbind();
        }

        //volume binding
        let volume_binding = obj
            .bind_property("volume", &adjustment, "value")
            .sync_create()
            .transform_to(move |_, value: &glib::Value| {
                let volume = value.get::<i32>().unwrap_or(0);
                Some(glib::Value::from(volume as f64))
            })
            .build();
        bindings.push(volume_binding);

        //+mute binding
    }

    pub fn bind_mic_volume_property(&self, obj: &AudioDeviceGObject) {
        let mut bindings = self.imp().mic_bindings.borrow_mut();
        let adjustment = self.imp().mic_volume.adjustment();

        for binding in bindings.drain(..) {
            binding.unbind();
        }

        //volume binding
        let volume_binding = obj
            .bind_property("volume", &adjustment, "value")
            .sync_create()
            .transform_to(move |_, value: &glib::Value| {
                let volume = value.get::<i32>().unwrap_or(0);
                Some(glib::Value::from(volume as f64))
            })
            .build();
        bindings.push(volume_binding);

        //+mute binding
    }

    pub fn setup_factory(&self, user_type: AudioDeviceUserType) {
        // Select the appropriate dropdown
        let switch = if user_type == AudioDeviceUserType::Speaker {
            self.imp().speaker_switch.get()
        } else {
            self.imp().mic_switch.get()
        };
        //setup factory
        let factory = SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            // Create `Label`
            let label = Label::new(None);
            label.set_property("halign", gtk::Align::Start);
            //label.add_css_class("dropdown-label");
            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&label));
        });

        // Tell factory how to bind `Label` to a `AudioDeviceGObject`
        factory.connect_bind(move |_, list_item| {
            // Get `AudioDeviceGObject` from `ListItem`
            let object = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<AudioDeviceGObject>()
                .expect("The item has to be an `AudioDeviceGObject`.");

            // Get `Label` from `ListItem`
            let label = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<Label>()
                .expect("The child has to be a `Label`.");

            label.set_label(&object.name());
        });

        factory.connect_unbind(move |_, list_item| {
            // Get `Label` from `ListItem`
            let label = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<Label>()
                .expect("The child has to be a `Label`.");

            label.set_label("");
        });

        switch.set_factory(Some(&factory));
    }

    //attempt to keep default row, no success yet
    pub fn setup_factory_2(&self, user_type: AudioDeviceUserType) {
        // Select the appropriate dropdown
        let switch = if user_type == AudioDeviceUserType::Speaker {
            self.imp().speaker_switch.get()
        } else {
            self.imp().mic_switch.get()
        };

        // Create the factory
        let factory = SignalListItemFactory::new();

        // Setup: Keep the default row, do not replace it
        factory.connect_setup(|_, _list_item| {});

        // Bind: Locate and update the correct label inside the row
        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<ListItem>()
                .expect("Expected a ListItem");

            if let Some(device) = list_item.item().and_downcast::<AudioDeviceGObject>() {
                if let Some(row) = list_item.child() {
                    // Use the recursive function to find the label
                    if let Some(label) = find_label(&row) {
                        label.set_label(&device.name());
                    }
                }
            }
        });

        // Unbind: Clear the label when the item is removed
        factory.connect_unbind(|_, list_item| {
            if let Some(row) = list_item.child() {
                if let Some(label) = find_label(&row) {
                    label.set_label(""); // Clear text
                }
            }
        });

        // Set the factory for the dropdown
        switch.set_factory(Some(&factory));
    }
}

// Recursive function to find the first Label inside a widget
fn find_label(widget: &gtk::Widget) -> Option<gtk::Label> {
    if let Some(label) = widget.downcast_ref::<gtk::Label>() {
        println!("Label child found!");
        return Some(label.clone());
    }

    for child in widget.first_child().into_iter() {
        if let Some(found_label) = find_label(&child) {
            println!("Label child found!");
            return Some(found_label);
        }
    }

    println!("No label child found");
    None
}
