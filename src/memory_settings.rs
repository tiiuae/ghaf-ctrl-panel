use glib::subclass::Signal;
use glib::{Binding, Properties};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Label};
use std::cell::RefCell;
use std::sync::OnceLock;

use crate::data_provider::StatsResponse;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::MemorySettings)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/memory_settings.ui")]
    pub struct MemorySettings {
        pub name: String,

        #[template_child]
        pub data_label: TemplateChild<Label>,

        #[property(name = "vm-name", get, set, type = String)]
        footer_visible: RefCell<String>,

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
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl MemorySettings {
        /*
        #[template_callback]
        fn on_advanced_settings_clicked(&self) {
            self.obj().open_advanced_settings_widget();
        }
        */
    } //end #[gtk::template_callbacks]

    #[glib::derived_properties]
    impl ObjectImpl for MemorySettings {
        fn constructed(&self) {
            self.parent_constructed();

            // After the object is constructed, bind the footer visibilty property
            let obj = self.obj();
            /*
            obj.bind_property("footer-visible", &self.footer.get(), "visible")
                .flags(glib::BindingFlags::DEFAULT)
                .build();
            */
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    /*
                    Signal::builder("mic-changed")
                        .param_types([u32::static_type()])
                        .build(),
                    */
                ]
            })
        }
    }

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
