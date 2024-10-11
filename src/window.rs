use std::rc::Rc;
use gtk::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate, Stack, Image, Button, ToggleButton, MenuButton, ListView, SingleSelection, CustomFilter, SignalListItemFactory, FilterListModel, ListItem,};
use glib::{Binding, Object, Variant};

use crate::application::ControlPanelGuiApplication;
use crate::vm_gobject::VMGObject;
use crate::vm_row::VMRow;
use crate::vm_settings::VMSettings;
use crate::settings::Settings;
use crate::vm_control_action::VMControlAction;
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
        pub services_view_button: TemplateChild<ToggleButton>,
        #[template_child]
        pub settings_view_button: TemplateChild<ToggleButton>,
        #[template_child]
        pub ghaf_logo: TemplateChild<Image>,

        #[template_child]
        pub stack: TemplateChild<Stack>,

        #[template_child]
        pub vm_list_view: TemplateChild<ListView>,
        #[template_child]
        pub vm_settings_box: TemplateChild<VMSettings>,

        #[template_child]
        pub settings_box: TemplateChild<Settings>,
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
        fn switch_to_vm_view(&self) {
            if (self.stack.visible_child_name() != Some("vm_view".into())) {
                self.stack.set_visible_child_name("vm_view");
            }
        }
        #[template_callback]
        fn switch_to_services_view(&self) {
            if (self.stack.visible_child_name() != Some("vm_view".into())) {
                self.stack.set_visible_child_name("vm_view");
            }
        }
        #[template_callback]
        fn switch_to_settings_view(&self) {
            if (self.stack.visible_child_name() != Some("settings_view".into())) {
                self.stack.set_visible_child_name("settings_view");
            }
        }

        #[template_callback]
        fn on_vm_control_action(&self, action: VMControlAction, name: String) {
            let app = self.obj().get_app_ref();
            app.control_vm(action, name);
        }
        #[template_callback]
        fn on_settings_action(&self, action: SettingsAction, value: Variant) {
            let app = self.obj().get_app_ref();
            app.perform_setting_action(action, value);
        }
    }//end #[gtk::template_callbacks]

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
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P) -> Self {
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

        self.setup_vm_rows();
        self.setup_factory();
        //vm view by default
        self.imp().vm_view_button.set_active(true);
    }

    #[inline(always)]
    fn get_app_ref(&self) -> Rc<ControlPanelGuiApplication> {
        let binding = self.application().expect("Failed to get application");     
        binding.downcast_ref::<ControlPanelGuiApplication>().expect("ControlPanelGuiApplication is expected!").clone().into()
    }

    fn setup_vm_rows(&self) {
        let app = self.get_app_ref();

        let model = app.get_store();//ListStore doc: "GLib type: GObject with reference counted clone semantics."

        self.imp().settings_box.set_vm_model(model.clone());

        //Create filter: VM services, default
        let vm_filter = CustomFilter::new(|item: &Object| {
            if let Some(vm_obj) = item.downcast_ref::<VMGObject>() {
                return vm_obj.is_app_vm();
            }
            false
        });

        //Create filter: other services
        let services_filter = CustomFilter::new(|item: &Object| {
            if let Some(vm_obj) = item.downcast_ref::<VMGObject>() {
                return !vm_obj.is_app_vm();
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
        selection_model.connect_items_changed(
            glib::clone!(@strong self as window => move |selection_model, position, removed, added| {
                println!("Items changed at position {}, removed: {}, added: {}", position, removed, added);
                if let Some(selected_item) = selection_model.selected_item() {
                    if let Some(vm_obj) = selected_item.downcast_ref::<VMGObject>() {
                        window.set_vm_details(&vm_obj);
                    }
                } else {
                    println!("No item selected");
                }
            })
        );
        
        self.imp().vm_list_view.set_model(Some(&selection_model));

        self.bind_vm_settings_box_visibility();

        //bind filter change
        let filter_model_clone = filter_model.clone();
        let selection_model_clone = selection_model.clone();
        let count = self.imp().vm_list_view.model().expect("no model!").n_items();
        self.imp().vm_view_button.connect_toggled(move |button| {
            if button.is_active() {
                println!("Filter is about to change to vm");
                filter_model_clone.set_filter(Some(&vm_filter));
                Self::set_default_selection(&selection_model_clone, count);
            }
        });
        self.imp().services_view_button.connect_toggled(move |button| {
            if button.is_active() {
                println!("Filter is about to change to services");
                filter_model.set_filter(Some(&services_filter));
                Self::set_default_selection(&selection_model, count);
            }
        });
    }

    fn set_default_selection(selection_model: &SingleSelection, count: u32) {
        println!("Selection is about to change");
        if (count <= 0) {return};
        selection_model.set_selected(0);
        selection_model.selection_changed(0u32, count);
    }

    fn bind_vm_settings_box_visibility(&self) {
        let imp = self.imp();
        let vm_settings_box = imp.vm_settings_box.clone().upcast::<gtk::Widget>();
        if let Some(model) = imp.vm_list_view.model() {
            model
            .bind_property("n_items", &vm_settings_box, "visible")
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
            let vm_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<VMRow>()
                .expect("The child has to be a `VMRow`.");

            vm_row.unbind();
        });

        // Set the factory of the list view
        self.imp().vm_list_view.set_factory(Some(&factory));
    }

    fn set_vm_details(&self, vm_obj: &VMGObject) {
        self.imp().vm_settings_box.bind(vm_obj);
    }
}
