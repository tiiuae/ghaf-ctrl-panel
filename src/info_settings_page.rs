use std::cell::RefCell;
use std::sync::OnceLock;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, ProgressBar, ListView, NoSelection, SignalListItemFactory, ListItem, CustomFilter, FilterListModel};
use glib::{Binding, Object};
use gtk::gio::ListStore;
use glib::subclass::Signal;

use crate::vm_gobject::VMGObject;
use crate::vm_row_2::VMRow2;
use crate::settings_gobject::SettingsGObject;
use givc_common::query::VMStatus;
use crate::vm_control_action::VMControlAction;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/info_settings_page.ui")]
    pub struct InfoSettingsPage {
        #[template_child]
        pub memory_bar: TemplateChild<ProgressBar>,
        #[template_child]
        pub cpu_bar: TemplateChild<ProgressBar>,
        #[template_child]
        pub network_bar: TemplateChild<ProgressBar>,
        #[template_child]
        pub vm_list_view: TemplateChild<ListView>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InfoSettingsPage {
        const NAME: &'static str = "InfoSettingsPage";
        type Type = super::InfoSettingsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            //klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for InfoSettingsPage {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            // Setup
            let obj = self.obj();
            obj.init();
        }
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("vm-control-action")
                    .param_types([VMControlAction::static_type(), String::static_type()])
                    .build(),
                    ]
            })
        }
    }
    impl WidgetImpl for InfoSettingsPage {}
    impl BoxImpl for InfoSettingsPage {}
}

glib::wrapper! {
pub struct InfoSettingsPage(ObjectSubclass<imp::InfoSettingsPage>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for InfoSettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl InfoSettingsPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
    pub fn init(&self) {
        //initial values to test styling
        self.imp().memory_bar.set_fraction(0.5);
        self.imp().cpu_bar.set_fraction(0.5);
        self.imp().network_bar.set_fraction(1.0);
    }

    pub fn set_vm_model(&self, model: ListStore) {
        self.setup_vm_rows(model.clone());
        self.setup_factory();
    }

    fn setup_vm_rows(&self, model: ListStore) {
        //Set filter: only running VM's
        let filter_model = FilterListModel::new(Some(model), Some(CustomFilter::new(|item: &Object| {
            if let Some(vm_obj) = item.downcast_ref::<VMGObject>() {
                if (vm_obj.is_app_vm() 
                    && (vm_obj.status() == (VMStatus::Running as u8))) {
                    return true;
                }
            }
            false
        })));

        // Wrap model with no selection and pass it to the list view
        let selection_model = NoSelection::new(Some(filter_model));
        self.imp().vm_list_view.set_model(Some(&selection_model));
    }

    fn setup_factory(&self) {
        // Create a new factory
        let factory = SignalListItemFactory::new();
        
        let this = self.clone();
        // Create an empty `VMRow2` during setup
        factory.connect_setup(move |_, list_item| {
            // Create `VMRow2`
            let vm_row = VMRow2::new();
            //connect signals
            let widget = this.clone();
            vm_row.connect_local(
                "vm-control-action",
                false,
                move |values| {
                    //the value[0] is self
                    let vm_action = values[1].get::<VMControlAction>().unwrap();
                    let vm_name = values[2].get::<String>().unwrap();
                    widget.emit_by_name::<()>("vm-control-action", &[&vm_action, &vm_name]);
                    None
                },
            );
            
            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&vm_row));
        });

        // Tell factory how to bind `VMRow2` to a `VMGObject`
        factory.connect_bind(move |_, list_item| {
            // Get `VMGObject` from `ListItem`
            let vm_object = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<VMGObject>()
                .expect("The item has to be an `VMGObject`.");

            // Get `VMRow2` from `ListItem`
            let vm_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<VMRow2>()
                .expect("The child has to be a `VMRow2`.");

            vm_row.bind(&vm_object);
        });

        // Tell factory how to unbind `VMRow2` from `VMGObject`
        factory.connect_unbind(move |_, list_item| {
            // Get `VMRow2` from `ListItem`
            let vm_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<VMRow2>()
                .expect("The child has to be a `VMRow2`.");

            vm_row.unbind();
        });

        // Set the factory of the list view
        self.imp().vm_list_view.set_factory(Some(&factory));
    }

    pub fn bind(&self, settings_object: &SettingsGObject) {
        //unbind previous ones
        self.unbind();
        //make new
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}

