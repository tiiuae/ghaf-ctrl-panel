use gio::ListStore;
use glib::subclass::Signal;
use glib::{Binding, Object, Variant};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{
    gio, glib, Box, CompositeTemplate, CustomFilter, DropDown, FilterListModel, Label, ListItem,
    Scale, SignalListItemFactory, SingleSelection,
};
use imp::AudioDeviceUserType;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::OnceLock;

use crate::audio_device_gobject::imp::AudioDeviceType;
use crate::audio_device_gobject::AudioDeviceGObject;
use crate::volume_widget::VolumeWidget;

mod imp {
    use super::*;

    #[derive(PartialEq)]
    pub enum AudioDeviceUserType {
        Mic = 0,
        Speaker = 1,
    }

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/audio_settings.ui")]
    pub struct AudioSettings {
        pub name: String,

        #[template_child]
        pub mic_switch: TemplateChild<DropDown>,
        #[template_child]
        pub mic_volume: TemplateChild<VolumeWidget>,
        #[template_child]
        pub speaker_switch: TemplateChild<DropDown>,
        #[template_child]
        pub speaker_volume: TemplateChild<VolumeWidget>,

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
        pub(super) fn on_mic_changed(&self, _: glib::ParamSpec, switch: &DropDown) {
            if let Some(obj) = switch
                .selected_item()
                .and_downcast_ref::<AudioDeviceGObject>()
            {
                let variant = (obj.id(), obj.dev_type()).to_variant();
                self.obj()
                    .emit_by_name::<()>("speaker-changed", &[&variant]);

                self.mic_volume.set_device(obj);
            }
        }

        #[template_callback]
        fn on_mic_deselected(&self) {
            glib::spawn_future_local(glib::clone!(
                #[strong(rename_to = widget)]
                self.obj(),
                async move {
                    // TODO: Need a better way to know when new default device has been selected
                    glib::timeout_future_seconds(1).await;
                    if let Some(input_model) = widget.imp().mic_switch.model() {
                        if let Some(idx) =
                            (0..).map_while(|i| input_model.item(i)).position(|obj| {
                                obj.downcast_ref::<AudioDeviceGObject>()
                                    .is_some_and(|obj| obj.is_default())
                            })
                        {
                            widget.imp().mic_switch.set_selected(idx as u32);
                        }
                    }
                }
            ));
        }

        #[template_callback]
        pub(super) fn on_speaker_changed(&self, _: glib::ParamSpec, switch: &DropDown) {
            if let Some(obj) = switch
                .selected_item()
                .and_downcast_ref::<AudioDeviceGObject>()
            {
                let variant = (obj.id(), obj.dev_type()).to_variant();
                self.obj()
                    .emit_by_name::<()>("speaker-changed", &[&variant]);

                self.speaker_volume.set_device(obj);
            }
        }

        #[template_callback]
        fn on_speaker_deselected(&self) {
            glib::spawn_future_local(glib::clone!(
                #[strong(rename_to = widget)]
                self.obj(),
                async move {
                    // TODO: Need a better way to know when new default device has been selected
                    glib::timeout_future_seconds(1).await;
                    if let Some(output_model) = widget.imp().speaker_switch.model() {
                        if let Some(idx) =
                            (0..).map_while(|i| output_model.item(i)).position(|obj| {
                                obj.downcast_ref::<AudioDeviceGObject>()
                                    .is_some_and(|obj| obj.is_default())
                            })
                        {
                            widget.imp().speaker_switch.set_selected(idx as u32);
                        }
                    }
                }
            ));
        }

        #[template_callback]
        fn on_mic_mute_changed(&self, _: glib::ParamSpec, volume: &VolumeWidget) {
            if let Some(obj) = self
                .mic_switch
                .selected_item()
                .and_downcast_ref::<AudioDeviceGObject>()
            {
                let value = volume.muted();
                if (value != obj.muted()) {
                    let variant = (obj.id(), obj.dev_type(), value).to_variant();
                    self.obj()
                        .emit_by_name::<()>("mic-mute-changed", &[&variant]);
                    #[cfg(feature = "mock")]
                    obj.set_muted(value);
                }
            }
        }

        #[template_callback]
        fn on_mic_volume_changed(&self, _: glib::ParamSpec, volume: &VolumeWidget) {
            if let Some(obj) = self
                .mic_switch
                .selected_item()
                .and_downcast_ref::<AudioDeviceGObject>()
            {
                let value = volume.volume();
                if (value != obj.volume()) {
                    let variant = (obj.id(), obj.dev_type(), value).to_variant();
                    self.obj()
                        .emit_by_name::<()>("mic-volume-changed", &[&variant]);
                    #[cfg(feature = "mock")]
                    obj.set_volume(value);
                }
            }
        }

        #[template_callback]
        fn on_speaker_mute_changed(&self, _: glib::ParamSpec, volume: &VolumeWidget) {
            if let Some(obj) = self
                .speaker_switch
                .selected_item()
                .and_downcast_ref::<AudioDeviceGObject>()
            {
                let value = volume.muted();
                if (value != obj.muted()) {
                    let variant = (obj.id(), obj.dev_type(), value).to_variant();
                    self.obj()
                        .emit_by_name::<()>("speaker-mute-changed", &[&variant]);
                    #[cfg(feature = "mock")]
                    obj.set_muted(value);
                }
            }
        }

        #[template_callback]
        fn on_speaker_volume_changed(&self, _: glib::ParamSpec, volume: &VolumeWidget) {
            if let Some(obj) = self
                .speaker_switch
                .selected_item()
                .and_downcast_ref::<AudioDeviceGObject>()
            {
                let value = volume.volume();
                if (value != obj.volume()) {
                    let variant = (obj.id(), obj.dev_type(), value).to_variant();
                    self.obj()
                        .emit_by_name::<()>("speaker-volume-changed", &[&variant]);
                    #[cfg(feature = "mock")]
                    obj.set_volume(value);
                }
            }
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for AudioSettings {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<[Signal; 7]> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                [
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
                    Signal::builder("mic-mute-changed")
                        .param_types([Variant::static_type()])
                        .build(),
                    Signal::builder("speaker-mute-changed")
                        .param_types([Variant::static_type()])
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

    pub fn set_audio_devices(&self, devices: ListStore) {
        //setup factory
        self.setup_factory(AudioDeviceUserType::Mic);
        self.setup_factory(AudioDeviceUserType::Speaker);

        //Create filter: outputs
        let outputs_filter = CustomFilter::new(|item: &Object| {
            item.downcast_ref().is_some_and(|obj: &AudioDeviceGObject| {
                obj.dev_type() == AudioDeviceType::Sink as i32
            })
        });

        //Create filter: inputs
        let inputs_filter = CustomFilter::new(|item: &Object| {
            item.downcast_ref().is_some_and(|obj: &AudioDeviceGObject| {
                obj.dev_type() == AudioDeviceType::Source as i32
            })
        });

        //setup model for outputs
        let output_model = FilterListModel::new(Some(devices.clone()), Some(outputs_filter));
        self.imp().speaker_switch.set_model(Some(&output_model));

        if let Some(idx) = (0..).map_while(|i| output_model.item(i)).position(|obj| {
            obj.downcast_ref::<AudioDeviceGObject>()
                .is_some_and(|obj| obj.is_default())
        }) {
            self.imp().speaker_switch.set_selected(idx as u32);
        }

        //setup model for inputs
        let input_model = FilterListModel::new(Some(devices), Some(inputs_filter));
        self.imp().mic_switch.set_model(Some(&input_model));
        //
        if let Some(idx) = (0..).map_while(|i| input_model.item(i)).position(|obj| {
            obj.downcast_ref::<AudioDeviceGObject>()
                .is_some_and(|obj| obj.is_default())
        }) {
            self.imp().mic_switch.set_selected(idx as u32);
        }
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
            if let Some(row) = list_item
                .downcast_ref::<ListItem>()
                .and_then(ListItem::child)
            {
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
