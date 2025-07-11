use gtk::gio::ListModel;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CustomFilter, FilterListModel, NoSelection};

use crate::prelude::*;
use crate::service_gobject::ServiceGObject;
use crate::vm_row::VMRow;
use crate::ControlPanelGuiWindow;
use std::fs;

mod imp {
    use glib::subclass::Signal;
    use glib::Binding;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate, Label, ListView, ProgressBar};
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use crate::control_action::ControlAction;
    use crate::plot::Plot;
    use crate::serie::Serie;
    use crate::service_gobject::ServiceGObject;

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

        #[template_child]
        pub cpu_sys_serie: TemplateChild<Serie>,
        #[template_child]
        pub cpu_user_serie: TemplateChild<Serie>,
        #[template_child]
        pub mem_used_serie: TemplateChild<Serie>,
        #[template_child]
        pub mem_needed_serie: TemplateChild<Serie>,
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
                    .param_types([ControlAction::static_type(), ServiceGObject::static_type()])
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
        let logging_id = fs::read_to_string("/etc/common/device-id")
            .map(std::borrow::Cow::from)
            .unwrap_or("Not found".into());
        self.imp()
            .device_id
            .set_text(&format!("Logging ID:    {logging_id}"));

        self.imp()
            .cpu_plot
            .set_view(None, None, Some(0.0), Some(1.0));
        self.imp()
            .cpu_plot
            .set_label_format(|f| format!("{pct:.0}%", pct = f * 100.));

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
                                .cpu_user_serie
                                .push(i, process.user_cycles as f32 / process.total_cycles as f32);
                            info.imp().cpu_sys_serie.push(
                                i,
                                (process.user_cycles + process.sys_cycles) as f32
                                    / process.total_cycles as f32,
                            );
                        }
                        if let Some(memory) = stats.memory {
                            info.imp().memory_plot.set_view(
                                None,
                                None,
                                Some(0.0),
                                Some(memory.total as f32),
                            );
                            info.imp()
                                .mem_used_serie
                                .push(i, (memory.total - memory.free) as f32);
                            info.imp()
                                .mem_needed_serie
                                .push(i, (memory.total - memory.available) as f32);
                        }
                        i += 1.;
                    }
                }
            }
        ));
    }

    pub fn set_vm_model(&self, model: impl IsA<ListModel>) {
        self.setup_service_rows(model);
        self.setup_factory();
    }

    fn setup_service_rows(&self, model: impl IsA<ListModel>) {
        //Set filter: only running VM's
        let filter_model = FilterListModel::new(
            Some(model),
            Some(CustomFilter::typed(ServiceGObject::is_vm_running)),
        );

        // Wrap model with no selection and pass it to the list view
        let selection_model = NoSelection::new(Some(filter_model));
        self.imp().vm_list_view.set_model(Some(&selection_model));
    }

    fn setup_factory(&self) {
        // Create a new factory
        let factory = TypedSignalListItemFactory::<ServiceGObject, VMRow>::new();

        let this = self.clone();
        // Create an empty `VMRow` during setup
        factory.on_setup(move |_| {
            let row = VMRow::new();
            let page = this.clone();
            row.connect_local("vm-control-action", false, move |values| {
                //the value[0] is self
                page.emit_by_name::<()>("vm-control-action", &[&values[1], &values[2]]);
                None
            });
            row
        });

        // Tell factory how to bind `VMRow` to a `ServiceGObject`
        factory.on_bind(move |_, row, obj| {
            row.bind(obj);
        });

        // Tell factory how to unbind `VMRow` from `ServiceGObject`
        factory.on_unbind(move |_, row| {
            row.unbind();
        });

        // Set the factory of the list view
        self.imp().vm_list_view.set_factory(Some(&*factory));
    }
}
