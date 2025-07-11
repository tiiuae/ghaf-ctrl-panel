use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::service_gobject::ServiceGObject;

mod imp {
    use glib::subclass::Signal;
    use glib::Binding;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate, Label, MenuButton, Popover};
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use crate::control_action::ControlAction;
    use crate::service_gobject::ServiceGObject;

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
        pub(super) bindings: RefCell<Vec<Binding>>,
        pub(super) object: RefCell<Option<ServiceGObject>>,
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
        fn emit_action(&self, action: ControlAction) {
            let Some(object) = self.object.borrow().clone() else {
                return;
            };
            self.obj()
                .emit_by_name::<()>("vm-control-action", &[&action, &object]);
            self.popover_menu.popdown();
        }

        #[template_callback]
        fn on_vm_restart_clicked(&self) {
            self.emit_action(ControlAction::Restart);
        }

        #[template_callback]
        fn on_vm_shutdown_clicked(&self) {
            self.emit_action(ControlAction::Shutdown);
        }

        #[template_callback]
        fn on_vm_pause_clicked(&self) {
            self.emit_action(ControlAction::Pause);
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for VMRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<[Signal; 1]> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                [Signal::builder("vm-control-action")
                    .param_types([ControlAction::static_type(), ServiceGObject::static_type()])
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
            .bind_property("display-name", &title, "label")
            .sync_create()
            .build();
        bindings.push(title_binding);

        let subtitle_binding = vm_object
            .bind_property("details", &subtitle, "label")
            .sync_create()
            .build();
        bindings.push(subtitle_binding);

        *self.imp().object.borrow_mut() = Some(vm_object.clone());
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
        *self.imp().object.borrow_mut() = None;
    }
}
