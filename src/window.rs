use std::cell::{Ref, RefCell};
use gtk::prelude::*;
use gio::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate, ListView, SingleSelection, SignalListItemFactory, ListItem, DropDown};

use crate::data_provider::imp::DataProvider;
use crate::vm_gobject::VMGObject;
use crate::vm_row::VMRow;
use crate::vm_settings::VMSettings;
use crate::audio_settings::AudioSettings;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/window.ui")]
    pub struct ControlPanelGuiWindow {
        // Template widgets
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub header_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub update_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub vm_view_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub settings_view_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub ghaf_logo: TemplateChild<gtk::Image>,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub vm_main_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub vm_list_view: TemplateChild<ListView>,
        #[template_child]
        pub vm_settings_box: TemplateChild<VMSettings>,

        #[template_child]
        pub settings_main_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub settings_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub details_label: TemplateChild<gtk::Label>,

        pub data_provider: DataProvider,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ControlPanelGuiWindow {
        const NAME: &'static str = "ControlPanelGuiWindow";
        type Type = super::ControlPanelGuiWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            // Register `VMRow`
            VMRow::ensure_type();

            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl ControlPanelGuiWindow {
        #[template_callback]
        fn on_update_clicked(&self) {
            self.data_provider.update_request();
    
        }
        #[template_callback]
        fn switch_to_vm_view(&self) {
            self.stack.set_visible_child_name("vm_view");
    
        }
        #[template_callback]
        fn switch_to_settings_view(&self) {
            self.stack.set_visible_child_name("settings_view");
        }

        #[template_callback]
        fn on_settings_row_selected(&self, row: &gtk::ListBoxRow) {
            if let Some(action_row) = row.downcast_ref::<adw::ActionRow>() {
                let title: Option<String> = action_row.property("title");
                if let Some(title) = title {
                    self.details_label.set_text(&format!("{title} settings will be here"));
                } else {
                    self.details_label.set_text("(No title)");
                }
            } else {
                self.details_label.set_text("(Invalid row type)");
            }
        }
    }//end #[gtk::template_callbacks]

    impl ObjectImpl for ControlPanelGuiWindow {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            // Setup
            let obj = self.obj();
            obj.setup_vm_rows();
            obj.setup_factory();
        }
    }

    impl WidgetImpl for ControlPanelGuiWindow {}
    impl WindowImpl for ControlPanelGuiWindow {}
    impl ApplicationWindowImpl for ControlPanelGuiWindow {}
    impl AdwApplicationWindowImpl for ControlPanelGuiWindow {}
}

glib::wrapper! {
    pub struct ControlPanelGuiWindow(ObjectSubclass<imp::ControlPanelGuiWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,        @implements gio::ActionGroup, gio::ActionMap;
}

impl ControlPanelGuiWindow {
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P) -> Self {
        let window: Self = glib::Object::builder()
            .property("application", application)
            .build();
        window.init();
        window
    }

    fn init(&self) {
        let imp = imp::ControlPanelGuiWindow::from_instance(self);

        /*
        // Connect signals here
        //Tab button signals
        imp.vm_view_button.connect_clicked(glib::clone!(@strong self as window => move |_| {
            window.switch_to_vm_view();
        }));
        imp.settings_view_button.connect_clicked(glib::clone!(@strong self as window => move |_| {
            window.switch_to_settings_view();
        }));
        // List box signals
        imp.list_box.connect_row_selected(glib::clone!(@strong self as window => move |_, row| {
            window.on_vm_list_row_selected(row);
        }));
        imp.settings_list_box.connect_row_selected(glib::clone!(@strong self as window => move |_, row| {
            window.on_settings_list_row_selected(row);
        }));
        */
    }

    fn vm_rows(&self) -> Ref<gio::ListStore> {
        // Get state
        self.imp().data_provider.get_store_ref()
    }

    fn setup_vm_rows(&self) {
        // Create new model
        let model = self.vm_rows();

        let count = model.n_items();//save count before model will be consumpted

        // Wrap model with selection and pass it to the list view
        let selection_model = SingleSelection::new(Some(model.clone()));//???
        // Connect to the selection-changed signal
        selection_model.connect_selection_changed(
            glib::clone!(@strong self as window => move |selection_model, _, _| {
                if let Some(selected_item) = selection_model.selected_item() {
                    println!("Selected: {}", selection_model.selected());
                    if let Some(vm_obj) = selected_item.downcast_ref::<VMGObject>() {//???
                        let title: Option<String> = vm_obj.property("name");
                        let subtitle: Option<String> = vm_obj.property("details");
                        println!("Property {}, {}", title.unwrap(), subtitle.unwrap());
                        window.set_vm_details(&vm_obj);
                    }
                } else {
                    println!("No item selected");
                }
            })
        );
        self.imp().vm_list_view.set_model(Some(&selection_model));

        //set default selection to 1st item
        selection_model.set_selected(0);
        selection_model.selection_changed(0u32, count);
    }

    fn setup_factory(&self) {
        // Create a new factory
        let factory = SignalListItemFactory::new();

        // Create an empty `VMRow` during setup
        factory.connect_setup(move |_, list_item| {
            // Create `VMRow`
            let vm_row = VMRow::new();
            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&vm_row));
        });

        // Tell factory how to bind `VMRow` to a `VMGObject`
        factory.connect_bind(move |_, list_item| {
            // Get `VMGObject` from `ListItem`
            let vm_object = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<VMGObject>()
                .expect("The item has to be an `VMGObject`.");

            // Get `VMRow` from `ListItem`
            let vm_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<VMRow>()
                .expect("The child has to be a `VMRow`.");

            vm_row.bind(&vm_object);
        });

        // Tell factory how to unbind `VMRow` from `VMGObject`
        factory.connect_unbind(move |_, list_item| {
            // Get `VMRow` from `ListItem`
            let task_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<VMRow>()
                .expect("The child has to be a `TaskRow`.");

            task_row.unbind();
        });

        // Set the factory of the list view
        self.imp().vm_list_view.set_factory(Some(&factory));
    }

    fn set_vm_details(&self, vm_obj: &VMGObject) {
        self.imp().vm_settings_box.bind(vm_obj);
    }

    //pub fn update_vm_list(&self, list: &Vec<VMObject>) {
    //}
}
