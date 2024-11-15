use glib::Binding;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use std::cell::RefCell;

use crate::security_icon::SecurityIcon;
use crate::service_gobject::ServiceGObject;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/service_row.ui")]
    pub struct ServiceRow {
        pub name: String,

        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub vm_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub security_icon: TemplateChild<gtk::Image>,

        // Vector holding the bindings to properties of `TaskObject`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServiceRow {
        const NAME: &'static str = "ServiceRow";
        type Type = super::ServiceRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ServiceRow {}
    impl WidgetImpl for ServiceRow {}
    impl BoxImpl for ServiceRow {}
}

glib::wrapper! {
pub struct ServiceRow(ObjectSubclass<imp::ServiceRow>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for ServiceRow {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceRow {
    pub fn new() -> Self {
        //glib::Object::new::<Self>()
        glib::Object::builder().build()
    }

    pub fn bind(&self, object: &ServiceGObject) {
        let title = self.imp().title_label.get();
        let subtitle = self.imp().subtitle_label.get();
        let security_icon = self.imp().security_icon.get();
        let mut bindings = self.imp().bindings.borrow_mut();
        let is_vm = object.is_vm();

        let name_property: &str = if is_vm { "display-name" } else { "name" };

        let title_binding = object
            .bind_property(name_property, &title, "label")
            //.bidirectional()
            .sync_create()
            .build();
        // Save binding
        bindings.push(title_binding);

        let subtitle_binding = object
            .bind_property("details", &subtitle, "label")
            .sync_create()
            .build();
        // Save binding
        bindings.push(subtitle_binding);

        let security_binding = object
            .bind_property("trust-level", &security_icon, "resource")
            .sync_create()
            .transform_to(move |_, value: &glib::Value| {
                let trust_level = value.get::<u8>().unwrap_or(0);
                Some(glib::Value::from(SecurityIcon::new(trust_level).0))
            })
            .build();
        // Save binding
        bindings.push(security_binding);

        //block was left here as example
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
