use glib::subclass::Signal;
use glib::{Binding, Object};
use gtk::gio::ListStore;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{
    glib, CompositeTemplate, CustomFilter, FilterListModel, Label, ListItem, ListView, NoSelection,
    ProgressBar, SignalListItemFactory,
};
use std::cell::RefCell;
use std::sync::OnceLock;

use crate::control_action::ControlAction;
use crate::plot::Plot;
use crate::serie::Serie;
use crate::service_gobject::ServiceGObject;
use crate::settings_gobject::SettingsGObject;
use crate::vm_row::VMRow;
use crate::window::ControlPanelGuiWindow;
use givc_common::query::VMStatus;
use std::fs;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/info_settings_page.ui")]
    pub struct InfoSettingsPage {
        #[template_child]
        pub device_id: TemplateChild<Label>,
        #[template_child]
        pub memory_plot: TemplateChild<Plot>,
        #[template_child]
        pub cpu_plot: TemplateChild<Plot>,
        #[template_child]
        pub network_bar: TemplateChild<ProgressBar>,
        #[template_child]
        pub vm_list_view: TemplateChild<ListView>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,

        pub cpu_serie: Serie,
        pub mem_serie: Serie,
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
                vec![Signal::builder("vm-control-action")
                    .param_types([ControlAction::static_type(), String::static_type()])
                    .build()]
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
        // Read device id
        let mut logging_id: String = "Logging ID:    ".to_owned();
        if let Ok(dev_id) = fs::read_to_string("/etc/common/device-id") {
            logging_id.push_str(&dev_id);
        } else {
            logging_id.push_str("Not found");
        }
        self.imp().device_id.set_text(&logging_id);

        //initial values to test styling
        self.imp().cpu_plot.add_serie(&self.imp().cpu_serie);
        self.imp()
            .cpu_plot
            .set_view(None, None, Some(0.0), Some(1.0));
        self.imp()
            .cpu_plot
            .set_label_format(|f| format!("{pct:.0}%", pct = f * 100.));

        self.imp().memory_plot.add_serie(&self.imp().mem_serie);
        self.imp().memory_plot.set_view(None, None, Some(0.0), None);
        self.imp()
            .memory_plot
            .set_label_format(|f| format!("{mb:.0} MB", mb = f / 1_048_576.));

        self.imp().network_bar.set_fraction(1.0);

        #[allow(clippy::cast_precision_loss)]
        glib::spawn_future_local(glib::clone!(
            #[strong(rename_to = info)]
            self,
            async move {
                let mut i = 1f32;
                if let Some(win) = info
                    .root()
                    .and_then(|root| root.downcast::<ControlPanelGuiWindow>().ok())
                {
                    let stats = win.get_stats("ghaf-host");
                    while let Ok(stats) = stats.recv().await {
                        if let Some(process) = stats.process {
                            info.imp()
                                .cpu_serie
                                .push(i, process.user_cycles as f32 / process.total_cycles as f32);
                        }
                        if let Some(memory) = stats.memory {
                            info.imp()
                                .mem_serie
                                .push(i, (memory.total - memory.available) as f32);
                        }
                        i += 1.;
                    }
                }
            }
        ));
    }

    pub fn set_vm_model(&self, model: ListStore) {
        self.setup_service_rows(model.clone());
        self.setup_factory();
    }

    fn setup_service_rows(&self, model: ListStore) {
        //Set filter: only running VM's
        let filter_model = FilterListModel::new(
            Some(model),
            Some(CustomFilter::new(|item: &Object| {
                if let Some(obj) = item.downcast_ref::<ServiceGObject>() {
                    if (obj.is_vm() && (obj.status() == (VMStatus::Running as u8))) {
                        return true;
                    }
                }
                false
            })),
        );

        // Wrap model with no selection and pass it to the list view
        let selection_model = NoSelection::new(Some(filter_model));
        self.imp().vm_list_view.set_model(Some(&selection_model));
    }

    fn setup_factory(&self) {
        // Create a new factory
        let factory = SignalListItemFactory::new();

        let this = self.clone();
        // Create an empty `VMRow` during setup
        factory.connect_setup(move |_, list_item| {
            // Create `VMRow`
            let service_row = VMRow::new();
            //connect signals
            let widget = this.clone();
            service_row.connect_local("vm-control-action", false, move |values| {
                //the value[0] is self
                let vm_action = values[1].get::<ControlAction>().unwrap();
                let vm_name = values[2].get::<String>().unwrap();
                widget.emit_by_name::<()>("vm-control-action", &[&vm_action, &vm_name]);
                None
            });

            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&service_row));
        });

        // Tell factory how to bind `VMRow` to a `ServiceGObject`
        factory.connect_bind(move |_, list_item| {
            // Get `ServiceGObject` from `ListItem`
            let service_object = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<ServiceGObject>()
                .expect("The item has to be an `ServiceGObject`.");

            // Get `VMRow` from `ListItem`
            let service_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<VMRow>()
                .expect("The child has to be a `VMRow`.");

            service_row.bind(&service_object);
        });

        // Tell factory how to unbind `VMRow` from `ServiceGObject`
        factory.connect_unbind(move |_, list_item| {
            // Get `VMRow` from `ListItem`
            let service_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<VMRow>()
                .expect("The child has to be a `VMRow`.");

            service_row.unbind();
        });

        // Set the factory of the list view
        self.imp().vm_list_view.set_factory(Some(&factory));
    }

    pub fn bind(&self, _settings_object: &SettingsGObject) {
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
