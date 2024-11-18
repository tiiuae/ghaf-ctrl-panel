use adw::subclass::prelude::*;
use gio::ListStore;
use glib::{Object, Variant};
use gtk::prelude::*;
use gtk::{
    gio, glib, CompositeTemplate, CustomFilter, FilterListModel, Image, ListItem, ListView,
    MenuButton, SignalListItemFactory, SingleSelection, Stack, ToggleButton,
};
use std::rc::Rc;

use crate::application::ControlPanelGuiApplication;
use crate::control_action::ControlAction;
use crate::service_gobject::ServiceGObject;
use crate::service_row::ServiceRow;
use crate::service_settings::ServiceSettings;
use crate::settings::Settings;
use crate::settings_action::SettingsAction;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/window.ui")]
    pub struct ControlPanelGuiWindow {
        // Template widgets
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub header_menu_button: TemplateChild<MenuButton>,
        #[template_child]
        pub vm_view_button: TemplateChild<ToggleButton>,
        #[template_child]
        pub app_view_button: TemplateChild<ToggleButton>,
        #[template_child]
        pub services_view_button: TemplateChild<ToggleButton>,
        #[template_child]
        pub settings_view_button: TemplateChild<ToggleButton>,
        #[template_child]
        pub ghaf_logo: TemplateChild<Image>,

        #[template_child]
        pub stack: TemplateChild<Stack>,

        #[template_child]
        pub services_list_view: TemplateChild<ListView>,
        #[template_child]
        pub service_settings_box: TemplateChild<ServiceSettings>,

        #[template_child]
        pub settings_box: TemplateChild<Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ControlPanelGuiWindow {
        const NAME: &'static str = "ControlPanelGuiWindow";
        type Type = super::ControlPanelGuiWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            // Register `ServiceRow`
            ServiceRow::ensure_type();

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
        fn switch_to_app_view(&self) {
            if (self.stack.visible_child_name() != Some("services_view".into())) {
                self.stack.set_visible_child_name("services_view");
            }
        }
        #[template_callback]
        fn switch_to_vm_view(&self) {
            if (self.stack.visible_child_name() != Some("services_view".into())) {
                self.stack.set_visible_child_name("services_view");
            }
        }
        #[template_callback]
        fn switch_to_services_view(&self) {
            if (self.stack.visible_child_name() != Some("services_view".into())) {
                self.stack.set_visible_child_name("services_view");
            }
        }
        #[template_callback]
        fn switch_to_settings_view(&self) {
            if (self.stack.visible_child_name() != Some("settings_view".into())) {
                self.stack.set_visible_child_name("settings_view");
            }
        }

        #[template_callback]
        fn on_control_action(&self, action: ControlAction, name: String) {
            let app = self.obj().get_app_ref();
            app.control_service(action, name);
        }
        #[template_callback]
        fn on_settings_action(&self, action: SettingsAction, value: Variant) {
            let app = self.obj().get_app_ref();
            app.perform_setting_action(action, value);
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for ControlPanelGuiWindow {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();
        }

        fn dispose(&self) {
            println!("Window destroyed!");
        }
    }

    impl WidgetImpl for ControlPanelGuiWindow {}
    impl WindowImpl for ControlPanelGuiWindow {}
    impl ApplicationWindowImpl for ControlPanelGuiWindow {}
    impl AdwApplicationWindowImpl for ControlPanelGuiWindow {}
}

glib::wrapper! {
    pub struct ControlPanelGuiWindow(ObjectSubclass<imp::ControlPanelGuiWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl ControlPanelGuiWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        let window: Self = glib::Object::builder()
            .property("application", application)
            .build();
        window.init();
        window
    }

    fn init(&self) {
        self.set_destroy_with_parent(true);

        self.connect_close_request(glib::clone!(@strong self as window => move |_| {
            println!("Close window request");
            let app = window.get_app_ref();
            app.clean_n_quit();
            glib::Propagation::Stop // Returning Stop allows the window to be destroyed
        }));

        self.connect_destroy(glib::clone!(@strong self as window => move |_| {
            println!("Destroy window");
        }));

        self.setup_service_rows();
        self.setup_factory();
        //vm view by default
        self.imp().vm_view_button.set_active(true);
    }

    #[inline(always)]
    fn get_app_ref(&self) -> Rc<ControlPanelGuiApplication> {
        let binding = self.application().expect("Failed to get application");
        binding
            .downcast_ref::<ControlPanelGuiApplication>()
            .expect("ControlPanelGuiApplication is expected!")
            .clone()
            .into()
    }

    fn setup_service_rows(&self) {
        let app = self.get_app_ref();

        let model = app.get_store(); //ListStore doc: "GLib type: GObject with reference counted clone semantics."

        self.imp().settings_box.set_vm_model(model.clone());

        //Create filter: VM services, default
        let vm_filter = CustomFilter::new(|item: &Object| {
            if let Some(obj) = item.downcast_ref::<ServiceGObject>() {
                return obj.is_vm();
            }
            false
        });

        //Create filter: Apps
        let app_filter = CustomFilter::new(|item: &Object| {
            if let Some(obj) = item.downcast_ref::<ServiceGObject>() {
                return obj.is_app();
            }
            false
        });

        //Create filter: other services
        let services_filter = CustomFilter::new(|item: &Object| {
            if let Some(obj) = item.downcast_ref::<ServiceGObject>() {
                return !obj.is_vm() && !obj.is_app();
            }
            false
        });

        //VM filter by default
        let filter_model = FilterListModel::new(Some(model), Some(vm_filter.clone()));

        // Wrap model with selection and pass it to the list view
        let selection_model = SingleSelection::new(Some(filter_model.clone()));
        // Connect to the selection-changed and items-changed signals
        selection_model.connect_selection_changed(
            glib::clone!(@strong self as window => move |selection_model, _, _| {
                if let Some(selected_item) = selection_model.selected_item() {
                    println!("Selected: {}", selection_model.selected());
                    if let Some(obj) = selected_item.downcast_ref::<ServiceGObject>() {//???
                        let title = obj.name();
                        let subtitle = obj.details();
                        println!("Property {title}, {subtitle}");
                        window.set_vm_details(&obj);
                    }
                } else {
                    println!("No item selected");
                }
            }),
        );
        selection_model.connect_items_changed(
            glib::clone!(@strong self as window => move |selection_model, position, removed, added| {
                println!("Items changed at position {}, removed: {}, added: {}", position, removed, added);
                if let Some(selected_item) = selection_model.selected_item() {
                    if let Some(obj) = selected_item.downcast_ref::<ServiceGObject>() {
                        window.set_vm_details(&obj);
                    }
                } else {
                    println!("No item selected");
                }
            })
        );

        self.imp()
            .services_list_view
            .set_model(Some(&selection_model));

        self.bind_service_settings_box_visibility();

        //bind filter change
        let filter_model_clone_vm = filter_model.clone();
        let selection_model_clone_vm = selection_model.clone();
        let filter_model_clone_app = filter_model.clone();
        let selection_model_clone_app = selection_model.clone();

        let count = self
            .imp()
            .services_list_view
            .model()
            .expect("no model!")
            .n_items();

        self.imp().vm_view_button.connect_toggled(move |button| {
            if button.is_active() {
                println!("Filter is about to change to vm");
                filter_model_clone_vm.set_filter(Some(&vm_filter));
                Self::set_default_selection(&selection_model_clone_vm, count);
            }
        });

        self.imp().app_view_button.connect_toggled(move |button| {
            if button.is_active() {
                println!("Filter is about to change to app");
                filter_model_clone_app.set_filter(Some(&app_filter));
                Self::set_default_selection(&selection_model_clone_app, count);
            }
        });

        self.imp()
            .services_view_button
            .connect_toggled(move |button| {
                if button.is_active() {
                    println!("Filter is about to change to services");
                    filter_model.set_filter(Some(&services_filter));
                    Self::set_default_selection(&selection_model, count);
                }
            });
    }

    fn set_default_selection(selection_model: &SingleSelection, count: u32) {
        println!("Selection is about to change");
        if (count <= 0) {
            return;
        };
        selection_model.set_selected(0);
        selection_model.selection_changed(0u32, count);
    }

    fn bind_service_settings_box_visibility(&self) {
        let imp = self.imp();
        let service_settings_box = imp.service_settings_box.clone().upcast::<gtk::Widget>();
        if let Some(model) = imp.services_list_view.model() {
            model
                .bind_property("n_items", &service_settings_box, "visible")
                .sync_create()
                .transform_to(move |_, value: &glib::Value| {
                    let count = value.get::<u32>().unwrap_or(0);
                    Some(glib::Value::from(count != 0))
                })
                .build();
        }
    }

    fn setup_factory(&self) {
        // Create a new factory
        let factory = SignalListItemFactory::new();

        // Create an empty `ServiceRow` during setup
        factory.connect_setup(move |_, list_item| {
            // Create `ServiceRow`
            let service_row = ServiceRow::new();
            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&service_row));
        });

        // Tell factory how to bind `ServiceRow` to a `ServiceGObject`
        factory.connect_bind(move |_, list_item| {
            // Get `ServiceGObject` from `ListItem`
            let object = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<ServiceGObject>()
                .expect("The item has to be an `ServiceGObject`.");

            // Get `ServiceRow` from `ListItem`
            let service_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ServiceRow>()
                .expect("The child has to be a `ServiceRow`.");

            service_row.bind(&object);
        });

        // Tell factory how to unbind `ServiceRow` from `ServiceGObject`
        factory.connect_unbind(move |_, list_item| {
            // Get `ServiceRow` from `ListItem`
            let service_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ServiceRow>()
                .expect("The child has to be a `ServiceRow`.");

            service_row.unbind();
        });

        // Set the factory of the list view
        self.imp().services_list_view.set_factory(Some(&factory));
    }

    fn set_vm_details(&self, obj: &ServiceGObject) {
        self.imp().service_settings_box.bind(obj);
    }

    //pub API
    pub fn restore_default_display_settings(&self) {
        self.imp().settings_box.restore_default_display_settings();
    }

    pub fn set_locale_model(&self, store: ListStore, selected: Option<usize>) {
        self.imp().settings_box.set_locale_model(store, selected);
    }

    pub fn set_timezone_model(&self, store: ListStore, selected: Option<usize>) {
        self.imp().settings_box.set_timezone_model(store, selected);
    }
}
