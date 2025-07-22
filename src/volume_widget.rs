use gtk::glib;

mod imp {
    #![allow(clippy::cast_possible_truncation)]
    use glib::subclass::Signal;
    use glib::{Binding, Properties, SignalHandlerId};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate, Scale, ToggleButton};
    use std::cell::{Cell, RefCell};
    use std::sync::OnceLock;

    use crate::audio_device_gobject::AudioDeviceGObject;

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::VolumeWidget)]
    #[template(resource = "/ae/tii/ghaf/controlpanelgui/ui/volume_widget.ui")]
    pub struct VolumeWidget {
        #[template_child]
        volume_scale: TemplateChild<Scale>,
        #[template_child]
        mute: TemplateChild<ToggleButton>,

        #[property(get, set = VolumeWidget::set_device)]
        device: RefCell<Option<AudioDeviceGObject>>,
        #[property(get)]
        muted: Cell<bool>,
        #[property(get)]
        volume: Cell<i32>,

        bindings: RefCell<Vec<Binding>>,
        notifies: RefCell<Vec<SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeWidget {
        const NAME: &'static str = "VolumeWidget";
        type Type = super::VolumeWidget;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for VolumeWidget {
        fn constructed(&self) {
            self.obj().set_sensitive(true);
            self.mute
                .bind_property("active", &*self.volume_scale, "sensitive")
                .sync_create()
                .transform_to(|_, active: bool| Some(!active))
                .build();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<[Signal; 1]> = OnceLock::new();
            SIGNALS.get_or_init(|| [Signal::builder("deselected").build()])
        }
    }
    impl BoxImpl for VolumeWidget {}
    impl WidgetImpl for VolumeWidget {}

    #[gtk::template_callbacks]
    impl VolumeWidget {
        fn set_device(&self, device: Option<AudioDeviceGObject>) {
            let mut bindings = self.bindings.borrow_mut();
            let mut notifies = self.notifies.borrow_mut();

            for binding in bindings.drain(..) {
                binding.unbind();
            }
            if let Some(device) = self.device.borrow().as_ref() {
                for notify in notifies.drain(..) {
                    device.disconnect(notify);
                }
            }
            self.obj().set_sensitive(device.is_some());
            if let Some(device) = device.as_ref() {
                self.volume.set(device.volume());
                self.muted.set(device.muted());
                self.volume_scale.set_value(f64::from(device.volume()));
                self.mute.set_active(!device.muted());

                bindings.extend([
                    device
                        .bind_property("volume", &self.volume_scale.adjustment(), "value")
                        .sync_create()
                        .transform_to(|_, volume: i32| Some(f64::from(volume)))
                        .build(),
                    device
                        .bind_property("muted", &*self.mute, "active")
                        .sync_create()
                        .build(),
                ]);
                notifies.push(device.connect_is_default_notify(glib::clone!(
                    #[strong(rename_to = volume)]
                    self.obj(),
                    move |dev| {
                        if !dev.is_default() {
                            volume.emit_by_name::<()>("deselected", &[]);
                        }
                    }
                )));
            }
            *self.device.borrow_mut() = device;
        }

        #[template_callback]
        fn on_volume_changed(&self, scale: &Scale) {
            self.volume.set(scale.value() as i32);
            self.obj().notify_volume();
        }

        #[template_callback]
        fn on_mute_changed(&self, mute: &ToggleButton) {
            self.muted.set(mute.is_active());
            self.obj().notify_muted();
        }
    }
}

glib::wrapper! {
    pub struct VolumeWidget(ObjectSubclass<imp::VolumeWidget>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Buildable;
}

impl Default for VolumeWidget {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl VolumeWidget {}
