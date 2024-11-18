use glib::subclass::Signal;
use glib::Binding;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Label, MenuButton, Popover};
use std::cell::RefCell;
use std::sync::OnceLock;

use crate::control_action::ControlAction;
use crate::service_gobject::ServiceGObject;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/vm_row.ui")]
    pub struct VMRow {
        #[template_child]
        pub title_label: TemplateChild<Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<Label>,
        #[template_child]
        pub vm_action_menu_button: TemplateChild<MenuButton>,
        #[template_child]
        pub popover_menu: TemplateChild<Popover>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VMRow {
        const NAME: &'static str = "VMRow";
        type Type = super::VMRow;
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
    impl VMRow {
        #[template_callback]
        fn on_vm_restart_clicked(&self) {
            let vm_name = self.title_label.label();
            //emit signal
            self.obj()
                .emit_by_name::<()>("vm-control-action", &[&ControlAction::Restart, &vm_name]);
            //and close menu
            self.popover_menu.popdown();
        }
        #[template_callback]
        fn on_vm_shutdown_clicked(&self) {
            let vm_name = self.title_label.label();
            self.obj()
                .emit_by_name::<()>("vm-control-action", &[&ControlAction::Shutdown, &vm_name]);
            self.popover_menu.popdown();
        }
        #[template_callback]
        fn on_vm_pause_clicked(&self) {
            let vm_name = self.title_label.label();
            self.obj()
                .emit_by_name::<()>("vm-control-action", &[&ControlAction::Pause, &vm_name]);
            self.popover_menu.popdown();
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for VMRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("vm-control-action")
                    .param_types([ControlAction::static_type(), String::static_type()])
                    .build()]
            })
        }
    }
    impl WidgetImpl for VMRow {}
    impl BoxImpl for VMRow {}
}

glib::wrapper! {
pub struct VMRow(ObjectSubclass<imp::VMRow>)
    @extends gtk::Widget, gtk::Box;
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

    pub fn bind(&self, vm_object: &ServiceGObject) {
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
