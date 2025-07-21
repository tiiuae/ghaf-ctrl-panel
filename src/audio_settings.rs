use gio::ListModel;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, CustomFilter, FilterListModel, Label};
use imp::AudioDeviceUserType;

use crate::audio_device_gobject::{AudioDeviceGObject, AudioDeviceType};
use crate::prelude::*;

mod imp {
    use glib::subclass::Signal;
    use glib::Binding;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate, DropDown};
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use crate::audio_device_gobject::{AudioDeviceGObject, AudioDeviceType};
    use crate::prelude::*;
    use crate::volume_widget::VolumeWidget;

    #[derive(Clone, Copy, PartialEq)]
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
        fn on_mic_changed(&self) {
            if let Some(ref obj) = self.mic_select().selected_obj() {
                self.obj()
                    .emit_by_name::<()>("mic-changed", &[&obj.id(), &obj.dev_type()]);

                self.mic_volume.set_device(obj);
            }
        }

        #[template_callback]
        fn on_speaker_changed(&self) {
            if let Some(ref obj) = self.speaker_select().selected_obj() {
                self.obj()
                    .emit_by_name::<()>("speaker-changed", &[&obj.id(), &obj.dev_type()]);

                self.speaker_volume.set_device(obj);
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
                    let mic = widget.imp().mic_select();
                    if let Some(idx) = mic
                        .model()
                        .and_then(|model| model.iter().position(|dev| dev.is_default()))
                    {
                        mic.set_selected(idx);
                    }
                }
            ));
        }

        #[template_callback]
        fn on_speaker_deselected(&self) {
            glib::spawn_future_local(glib::clone!(
                #[strong(rename_to = widget)]
                self.obj(),
                async move {
                    // TODO: Need a better way to know when new default device has been selected
                    glib::timeout_future_seconds(1).await;
                    let speaker = widget.imp().mic_select();
                    if let Some(idx) = speaker
                        .model()
                        .and_then(|model| model.iter().position(|dev| dev.is_default()))
                    {
                        speaker.set_selected(idx);
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
                if value != obj.muted() {
                    self.obj().emit_by_name::<()>(
                        "mic-mute-changed",
                        &[&obj.id(), &obj.dev_type(), &value],
                    );
                    #[cfg(feature = "mock")]
                    obj.set_muted(value);
                }
            }
        }

        #[template_callback]
        fn on_mic_volume_changed(&self, _: glib::ParamSpec, volume: &VolumeWidget) {
            if let Some(ref obj) = self.mic_switch.selected_obj::<AudioDeviceGObject>() {
                #[allow(clippy::cast_possible_truncation)]
                let value = volume.volume();
                if value != obj.volume() {
                    self.obj().emit_by_name::<()>(
                        "mic-volume-changed",
                        &[&obj.id(), &obj.dev_type(), &value],
                    );
                    //save new value
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
                if value != obj.muted() {
                    self.obj().emit_by_name::<()>(
                        "speaker-mute-changed",
                        &[&obj.id(), &obj.dev_type(), &value],
                    );
                    #[cfg(feature = "mock")]
                    obj.set_muted(value);
                }
            }
        }

        #[template_callback]
        fn on_speaker_volume_changed(&self, _: glib::ParamSpec, volume: &VolumeWidget) {
            if let Some(ref obj) = self.speaker_switch.selected_obj::<AudioDeviceGObject>() {
                #[allow(clippy::cast_possible_truncation)]
                let value = volume.volume();
                if value != obj.volume() {
                    self.obj().emit_by_name::<()>(
                        "speaker-volume-changed",
                        &[&obj.id(), &obj.dev_type(), &value],
                    );
                    //save new value
                    #[cfg(feature = "mock")]
                    obj.set_volume(value);
                }
            }
        }

        pub(super) fn mic_select(&self) -> TypedDropDown<AudioDeviceGObject> {
            TypedDropDown::from((*self.mic_switch).clone())
        }

        pub(super) fn speaker_select(&self) -> TypedDropDown<AudioDeviceGObject> {
            TypedDropDown::from((*self.speaker_switch).clone())
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
                        .param_types([i32::static_type(), AudioDeviceType::static_type()])
                        .build(),
                    Signal::builder("speaker-changed")
                        .param_types([i32::static_type(), AudioDeviceType::static_type()])
                        .build(),
                    Signal::builder("mic-volume-changed")
                        .param_types([
                            i32::static_type(),
                            AudioDeviceType::static_type(),
                            i32::static_type(),
                        ])
                        .build(),
                    Signal::builder("speaker-volume-changed")
                        .param_types([
                            i32::static_type(),
                            AudioDeviceType::static_type(),
                            i32::static_type(),
                        ])
                        .build(),
                    Signal::builder("mic-mute-changed")
                        .param_types([
                            i32::static_type(),
                            AudioDeviceType::static_type(),
                            bool::static_type(),
                        ])
                        .build(),
                    Signal::builder("speaker-mute-changed")
                        .param_types([
                            i32::static_type(),
                            AudioDeviceType::static_type(),
                            bool::static_type(),
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

    pub fn set_audio_devices(&self, devices: impl IsA<ListModel>) {
        //setup factory
        self.setup_factory(AudioDeviceUserType::Mic);
        self.setup_factory(AudioDeviceUserType::Speaker);

        //Create filter: outputs
        let outputs_filter =
            CustomFilter::typed(|obj: &AudioDeviceGObject| obj.dev_type() == AudioDeviceType::Sink);

        //Create filter: inputs
        let inputs_filter = CustomFilter::typed(|obj: &AudioDeviceGObject| {
            obj.dev_type() == AudioDeviceType::Source
        });

        //setup model for outputs
        let output_model = FilterListModel::new(Some(devices.clone()), Some(outputs_filter)).wrap();
        self.imp().speaker_select().set_model(&output_model);

        if let Some(idx) = output_model.iter().position(|dev| dev.is_default()) {
            self.imp().speaker_select().set_selected(idx);
        }

        //setup model for inputs
        let input_model = FilterListModel::new(Some(devices), Some(inputs_filter)).wrap();
        self.imp().mic_select().set_model(&input_model);

        if let Some(idx) = input_model.iter().position(|dev| dev.is_default()) {
            self.imp().mic_select().set_selected(idx);
        }
    }

    pub fn setup_factory(&self, user_type: AudioDeviceUserType) {
        // Select the appropriate dropdown
        let switch = match user_type {
            AudioDeviceUserType::Speaker => self.imp().speaker_switch.get(),
            AudioDeviceUserType::Mic => self.imp().mic_switch.get(),
        };
        //setup factory
        let factory = TypedSignalListItemFactory::<AudioDeviceGObject, Label>::new();

        factory.on_setup(move |_| {
            // Create `Label`
            let label = Label::new(None);
            label.set_halign(gtk::Align::Start);
            label
        });

        // Tell factory how to bind `Label` to a `AudioDeviceGObject`
        factory.on_bind(move |_, label, item| {
            label.set_label(&item.name());
        });

        factory.on_unbind(move |_, label| {
            label.set_label("");
        });

        switch.set_factory(Some(&*factory));
    }
}
