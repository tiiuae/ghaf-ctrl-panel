use dbus::blocking::{Connection, Proxy};
use gio::ListStore;
use glib::subclass::Signal;
use glib::{Binding, Object, Properties};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{
    gio, glib, Box, CompositeTemplate, CustomFilter, DropDown, FilterListModel, Label, ListItem,
    Scale, SignalListItemFactory, SingleSelection,
};
use imp::AudioDeviceUserType;
use std::cell::RefCell;
use std::sync::OnceLock;
use std::time::Duration;

use crate::audio_device_gobject::imp::AudioDeviceType;
use crate::audio_device_gobject::AudioDeviceGObject;

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
        fn on_advanced_settings_clicked(&self) {
            self.obj().open_advanced_settings_widget();
        }
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
            self.obj()
                .emit_by_name::<()>("mic-volume-changed", &[&value]);
        }
        #[template_callback]
        fn on_speaker_volume_changed(&self, scale: &Scale) {
            let value = scale.value();
            self.obj()
                .emit_by_name::<()>("speaker-volume-changed", &[&value]);
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
            self.obj()
                .emit_by_name::<()>("apply-new", &[&mic, &speaker, &mic_volume, &speaker_volume]);
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
                    Signal::builder("set-defaults").build(),
                    Signal::builder("apply-new")
                        .param_types([
                            u32::static_type(),
                            u32::static_type(),
                            f64::static_type(),
                            f64::static_type(),
                        ])
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

    pub fn set_audio_devices(&self, devices: ListStore) {
        //setup factory
        self.setup_factory(AudioDeviceUserType::Mic);
        self.setup_factory(AudioDeviceUserType::Speaker);

        //Create filter: outputs
        let outputs_filter = CustomFilter::new(|item: &Object| {
            if let Some(obj) = item.downcast_ref::<AudioDeviceGObject>() {
                return (obj.dev_type() == AudioDeviceType::Sink as i32)
                    || (obj.dev_type() == AudioDeviceType::SourceOutput as i32);
            }
            false
        });

        //Create filter: inputs
        let inputs_filter = CustomFilter::new(|item: &Object| {
            if let Some(obj) = item.downcast_ref::<AudioDeviceGObject>() {
                return (obj.dev_type() == AudioDeviceType::Source as i32)
                    || (obj.dev_type() == AudioDeviceType::SinkInput as i32);
            }
            false
        });

        let count = devices.n_items();
        println!("Devices came to audio settings: {count}");

        //setup model for outputs
        let output_filter_model =
            FilterListModel::new(Some(devices.clone()), Some(outputs_filter));
        let output_model = SingleSelection::new(Some(output_filter_model));

        //setup model for inputs
        let input_filter_model = FilterListModel::new(Some(devices), Some(inputs_filter));
        let input_model = SingleSelection::new(Some(input_filter_model));

        output_model.connect_selection_changed(
            glib::clone!(@strong self as widget => move |selection_model, _, _| {
                if let Some(selected_item) = selection_model.selected_item() {
                    println!("Selected: {}", selection_model.selected());
                    if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                        widget.imp().speaker_volume.set_value(obj.volume() as f64);
                    }
                } else {
                    println!("No item selected");
                }
            }),
        );
        output_model.connect_items_changed(
            glib::clone!(@strong self as widget => move |selection_model, position, removed, added| {
                println!("Items changed at position {}, removed: {}, added: {}", position, removed, added);
                if let Some(selected_item) = selection_model.selected_item() {
                    if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                        widget.imp().speaker_volume.set_value(obj.volume() as f64);
                    }
                } else {
                    println!("No item selected");
                }
            })
        );
        input_model.connect_selection_changed(
            glib::clone!(@strong self as widget => move |selection_model, _, _| {
                if let Some(selected_item) = selection_model.selected_item() {
                    println!("Selected: {}", selection_model.selected());
                    if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                        widget.imp().mic_volume.set_value(obj.volume() as f64);
                    }
                } else {
                    println!("No item selected");
                }
            }),
        );
        input_model.connect_items_changed(
            glib::clone!(@strong self as widget => move |selection_model, position, removed, added| {
                println!("Items changed at position {}, removed: {}, added: {}", position, removed, added);
                if let Some(selected_item) = selection_model.selected_item() {
                    if let Some(obj) = selected_item.downcast_ref::<AudioDeviceGObject>() {
                        widget.imp().mic_volume.set_value(obj.volume() as f64);
                    }
                } else {
                    println!("No item selected");
                }
            })
        );
        self.imp().speaker_switch.set_model(Some(&output_model));
        self.imp().mic_switch.set_model(Some(&input_model));

        //set default selection
        input_model.selection_changed(0u32, input_model.n_items());
        output_model.selection_changed(0u32, output_model.n_items());
    }

    pub fn setup_factory(&self, user_type: AudioDeviceUserType) {
        let (switch, volume) = if user_type == AudioDeviceUserType::Speaker {
            (
                self.imp().speaker_switch.get(),
                self.imp().speaker_volume.get(),
            )
        } else {
            (self.imp().mic_switch.get(), self.imp().mic_volume.get())
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
            volume.set_value(object.volume() as f64);
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
        // Select the correct dropdown and volume control
        let (switch, volume) = if user_type == AudioDeviceUserType::Speaker {
            (self.imp().speaker_switch.get(), self.imp().speaker_volume.get())
        } else {
            (self.imp().mic_switch.get(), self.imp().mic_volume.get())
        };
    
        // Create the factory
        let factory = SignalListItemFactory::new();
    
        // Setup: Keep the default row, do not replace it
        factory.connect_setup(|_, _list_item| {});
    
        // Bind: Locate and update the correct label inside the row
        let volume_clone = volume.clone();
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
    
                // Update the correct volume slider
                volume_clone.set_value(device.volume() as f64);
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

    pub fn open_advanced_settings_widget(&self) {
        // Connect to the session bus
        let connection = match Connection::new_session() {
            Ok(connection) => connection,
            Err(e) => {
                eprintln!("Failed to connect to session bus: {}", e);
                return;
            }
        };

        // Create a proxy to the object and interface
        let proxy = Proxy::new(
            "org.ghaf.Audio",            // Service name
            "/org/ghaf/Audio",           // Object path
            Duration::from_millis(5000), // Timeout for the method call
            &connection,
        );

        // Make the method call
        match proxy.method_call("org.ghaf.Audio", "Open", ()) {
            Ok(()) => println!("D-Bus message has been sent successfully."),
            Err(e) => eprintln!("Failed to send D-Bus message: {}", e),
        }
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
