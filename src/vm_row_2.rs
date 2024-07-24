use std::cell::RefCell;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use glib::Binding;

use crate::vm_gobject::VMGObject;
use crate::security_icon::SecurityIcon;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/vm_row_2.ui")]
    pub struct VMRow2 {
        pub name: String,

        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VMRow2 {
        const NAME: &'static str = "VMRow2";
        type Type = super::VMRow2;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
                klass.bind_template();
                //klass.bind_template_callbacks(); !!!
            }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VMRow2 {}
    impl WidgetImpl for VMRow2 {}
    impl BoxImpl for VMRow2 {}
}

glib::wrapper! {
pub struct VMRow2(ObjectSubclass<imp::VMRow2>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for VMRow2 {
    fn default() -> Self {
        Self::new()
    }
}

impl VMRow2 {
    pub fn new() -> Self {
        //glib::Object::new::<Self>()
        glib::Object::builder().build()
    }

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

