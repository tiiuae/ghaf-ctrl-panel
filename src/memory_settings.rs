use glib::Binding;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Label};
use std::cell::RefCell;

use crate::data_provider::StatsResponse;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/memory_settings.ui")]
    pub struct MemorySettings {
        pub name: String,

        #[template_child]
        pub data_label: TemplateChild<Label>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MemorySettings {
        const NAME: &'static str = "MemorySettings";
        type Type = super::MemorySettings;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MemorySettings {}
    impl WidgetImpl for MemorySettings {}
    impl BoxImpl for MemorySettings {}
}

glib::wrapper! {
pub struct MemorySettings(ObjectSubclass<imp::MemorySettings>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self::new()
    }
}

impl MemorySettings {
    pub fn new() -> Self {
        //glib::Object::new::<Self>()
        glib::Object::builder().build()
    }

    pub fn show_data(&self, stats: &StatsResponse) {
        let Some(mem) = stats.memory else {
            self.imp().data_label.set_label("");
            return;
        };
        self.imp().data_label.set_markup(&format!(
            "<b>Total:</b> {:.1} GB\n\
            <b>Available:</b> {:.1} GB",
            mem.total as f32 / 1073741824.,
            mem.available as f32 / 1073741824.,
        ));
    }

    pub fn clear_data(&self) {
        self.imp().data_label.set_label("");
    }
}
