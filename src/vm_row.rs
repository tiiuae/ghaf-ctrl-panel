use std::cell::RefCell;
use gtk::prelude::*;
//use adw::prelude::*;
//use adw::ActionRow;
use gtk::subclass::prelude::*;//gtk <-> adw
use gtk::{glib, CompositeTemplate};
use glib::Binding;

use crate::vm_gobject::VMGObject;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/vm_row.ui")]
    pub struct VMRow {
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
    impl ObjectSubclass for VMRow {
        const NAME: &'static str = "VMRow";
        type Type = super::VMRow;
        type ParentType = gtk::Box;//adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
                klass.bind_template();
            }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VMRow {}
    impl WidgetImpl for VMRow {}
    impl BoxImpl for VMRow {}
    //impl ListBoxRowImpl for VMRow {}
    //impl PreferencesRowImpl for VMRow {}
    //impl ActionRowImpl for VMRow {}
}

glib::wrapper! {
pub struct VMRow(ObjectSubclass<imp::VMRow>)
    @extends gtk::Widget, gtk::Box;//, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
    //@implements gtk::Accessible, gtk::Buildable; ???
}

impl Default for VMRow {
    fn default() -> Self {
        Self::new()
    }
}

impl VMRow {
    pub fn new() -> Self {
        //glib::Object::new::<Self>()
        glib::Object::builder().build()
    }

    pub fn set_data(&self) {
        println!("{}", self.imp().name);//access the struct's field'
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

