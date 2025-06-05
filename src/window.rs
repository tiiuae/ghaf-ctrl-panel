use adw::subclass::prelude::*;
use gio::ListModel;
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::application::ControlPanelGuiApplication;
pub use crate::application::StatsResponse;
use crate::prelude::*;

mod imp {
    use adw::subclass::prelude::*;
    use gio::ListModel;
    use gtk::prelude::*;
    use gtk::{
        gio, glib, CompositeTemplate, CustomFilter, FilterListModel, Image, ListView, MenuButton,
        SingleSelection, Stack, ToggleButton,
    };

    use crate::control_action::ControlAction;
    use crate::prelude::*;
    use crate::service_gobject::ServiceGObject;
    use crate::service_row::ServiceRow;
    use crate::service_settings::ServiceSettings;
    use crate::settings::Settings;
    use crate::settings_action::SettingsAction;

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
            if self.stack.visible_child_name() != Some("services_view".into()) {
                self.stack.set_visible_child_name("services_view");
            }
        }

        #[template_callback]
        fn switch_to_vm_view(&self) {
            if self.stack.visible_child_name() != Some("services_view".into()) {
                self.stack.set_visible_child_name("services_view");
            }
        }

        #[template_callback]
        fn switch_to_services_view(&self) {
            if self.stack.visible_child_name() != Some("services_view".into()) {
                self.stack.set_visible_child_name("services_view");
            }
        }

        #[template_callback]
        fn switch_to_settings_view(&self) {
            if self.stack.visible_child_name() != Some("settings_view".into()) {
                self.stack.set_visible_child_name("settings_view");
            }
        }

        #[template_callback]
        fn on_control_action(&self, action: ControlAction, object: ServiceGObject) {
            let app = self.obj().get_app_ref();
            app.control_service(action, object);
        }

        #[template_callback]
        fn on_settings_action(&self, action: SettingsAction) {
            let app = self.obj().get_app_ref();
            app.perform_setting_action(action);
        }

        pub fn setup_service_rows(&self, model: ListModel) {
            self.settings_box.set_vm_model(model.clone());

            //Create filter: VM services, default
            let vm_filter = CustomFilter::typed(ServiceGObject::is_vm);

            //Create filter: Apps
            let app_filter = CustomFilter::typed(ServiceGObject::is_app);

            //Create filter: other services
            let services_filter = CustomFilter::typed(ServiceGObject::is_service);

            //VM filter by default
            let filter_model = FilterListModel::new(Some(model), Some(vm_filter.clone()));

            // Wrap model with selection and pass it to the list view
            let selection_model =
                SingleSelection::new(Some(filter_model.clone())).wrap::<ServiceGObject>();
            // Connect to the selection-changed and items-changed signals
            selection_model.connect_selection_changed(glib::clone!(
                #[strong(rename_to = window)]
                self.obj(),
                #[strong]
                selection_model,
                move |_, _, _| {
                    if let Some(obj) = selection_model.selected_obj() {
                        let title = obj.name();
                        let subtitle = obj.details();
                        debug!("Property {title}, {subtitle}");
                        window.imp().set_vm_details(&obj);
                    } else {
                        debug!("No item selected");
                    }
                }
            ));
            selection_model.connect_items_changed(glib::clone!(
                #[strong(rename_to = window)]
                self.obj(),
                move |selection_model, position, removed, added| {
                    debug!(
                        "Items changed at position {position}, removed: {removed}, added: {added}"
                    );
                    if let Some(obj) = selection_model.selected_obj() {
                        window.imp().set_vm_details(&obj);
                    } else {
                        debug!("No item selected");
                    }
                }
            ));

            self.services_list_view.set_model(Some(&*selection_model));

            self.bind_service_settings_box_visibility();

            //bind filter change
            let filter_model_clone_vm = filter_model.clone();
            let selection_model_clone_vm = selection_model.clone();
            let filter_model_clone_app = filter_model.clone();
            let selection_model_clone_app = selection_model.clone();

            let count = self
                .services_list_view
                .model()
                .expect("no model!")
                .n_items();

            self.vm_view_button.connect_toggled(move |button| {
                if button.is_active() {
                    debug!("Filter is about to change to vm");
                    filter_model_clone_vm.set_filter(Some(&vm_filter));
                    Self::set_default_selection(&selection_model_clone_vm, count);
                }
            });

            self.app_view_button.connect_toggled(move |button| {
                if button.is_active() {
                    debug!("Filter is about to change to app");
                    filter_model_clone_app.set_filter(Some(&app_filter));
                    Self::set_default_selection(&selection_model_clone_app, count);
                }
            });

            self.services_view_button.connect_toggled(move |button| {
                if button.is_active() {
                    debug!("Filter is about to change to services");
                    filter_model.set_filter(Some(&services_filter));
                    Self::set_default_selection(&selection_model, count);
                }
            });
        }

        fn bind_service_settings_box_visibility(&self) {
            let service_settings_box = self.service_settings_box.clone().upcast::<gtk::Widget>();
            if let Some(model) = self.services_list_view.model() {
                model
                    .bind_property("n_items", &service_settings_box, "visible")
                    .sync_create()
                    .transform_to(move |_, count: u32| Some(count != 0))
                    .build();
            }
        }

        fn set_default_selection(selection_model: &SingleSelection, count: u32) {
            debug!("Selection is about to change");
            if count == 0 {
                return;
            }
            selection_model.set_selected(0);
            selection_model.selection_changed(0u32, count);
        }

        pub fn setup_factory(&self) {
            let factory = TypedSignalListItemFactory::<ServiceGObject, ServiceRow>::new();

            factory.on_setup(|_| ServiceRow::new());
            factory.on_bind(move |_, row, obj| row.bind(obj));
            factory.on_unbind(|_, row| row.unbind());

            // Set the factory of the list view
            self.services_list_view.set_factory(Some(&*factory));
        }

        fn set_vm_details(&self, obj: &ServiceGObject) {
            self.service_settings_box.bind(obj);
        }

        pub(super) fn set_audio_devices(&self, devices: ListModel) {
            self.settings_box.set_audio_devices(devices);
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for ControlPanelGuiWindow {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();
        }

        fn dispose(&self) {
            debug!("Window destroyed!");
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
        @implements gio::ActionGroup, gio::ActionMap, gtk::Root;
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
        let app = self.get_app_ref();

        self.connect_close_request(glib::clone!(
            #[strong]
            app,
            move |_| {
                debug!("Close window request");
                app.activate_action("quit", None);
                glib::Propagation::Stop // Returning Stop allows the window to be destroyed
            }
        ));

        self.connect_destroy(|_| {
            debug!("Destroy window");
        });

        //get application reference

        self.imp().setup_service_rows(app.get_model()); //ListStore doc: "GLib type: GObject with reference counted clone semantics."
        self.imp().setup_factory();
        //vm view by default
        self.imp().vm_view_button.set_active(true);
    }

    #[inline]
    fn get_app_ref(&self) -> ControlPanelGuiApplication {
        let binding = self.application().expect("Failed to get application");
        binding
            .downcast()
            .expect("ControlPanelGuiApplication is expected!")
    }

    pub fn get_stats(&self, vm: impl Into<String>) -> async_channel::Receiver<StatsResponse> {
        let (tx, rx) = async_channel::bounded(10);
        let vm = vm.into();

        glib::spawn_future_local(glib::clone!(
            #[strong(rename_to = win)]
            self,
            async move {
                let app = win.get_app_ref();
                loop {
                    if let Ok(stats) = app.get_stats(vm.clone()).await {
                        if tx.send(stats).await.is_err() {
                            break;
                        }
                    }
                    glib::timeout_future_seconds(5).await;
                }
            }
        ));

        rx
    }

    //pub API
    pub fn restore_default_display_settings(&self) {
        self.imp().settings_box.restore_default_display_settings();
    }

    pub fn set_locale_model(&self, model: impl IsA<ListModel>, selected: Option<usize>) {
        self.imp().settings_box.set_locale_model(model, selected);
    }

    pub fn set_timezone_model(&self, model: impl IsA<ListModel>, selected: Option<usize>) {
        self.imp().settings_box.set_timezone_model(model, selected);
    }

    pub fn set_audio_devices(&self, devices: ListModel) {
        self.imp().set_audio_devices(devices);
    }
}
