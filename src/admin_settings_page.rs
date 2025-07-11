use gtk::glib;

mod imp {
    use crate::prelude::*;
    use glib::subclass::Signal;
    use glib::Binding;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, Box, CompositeTemplate, Label, Notebook};
    use std::cell::RefCell;
    use std::sync::OnceLock;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/admin_settings_page.ui")]
    pub struct AdminSettingsPage {
        #[template_child]
        pub tab_widget: TemplateChild<Notebook>,
        #[template_child]
        pub update_content: TemplateChild<Box>,
        #[template_child]
        pub update_status_label: TemplateChild<Label>,
        #[template_child]
        pub update_version_label: TemplateChild<Label>,
        #[template_child]
        pub update_size_label: TemplateChild<Label>,
        #[template_child]
        pub update_button_label: TemplateChild<Label>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AdminSettingsPage {
        const NAME: &'static str = "AdminSettingsPage";
        type Type = super::AdminSettingsPage;
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
    impl AdminSettingsPage {
        #[template_callback]
        fn on_update_clicked(&self) {
            debug!("Update clicked!");
            self.obj().emit_by_name::<()>("update-request", &[]);
            //check for updates and get the result
            //mock
            self.update_status_label.set_label("Update available");
            self.update_version_label.set_label("Version: 1.0.1");
            self.update_size_label.set_label("Size: 100 Mb");
            self.update_button_label.set_label("Update");
            self.update_content.set_visible(true);
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for AdminSettingsPage {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();
        }
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("check-for-update-request").build(),
                    Signal::builder("update-request").build(),
                ]
            })
        }
    }
    impl WidgetImpl for AdminSettingsPage {}
    impl BoxImpl for AdminSettingsPage {}
}

glib::wrapper! {
pub struct AdminSettingsPage(ObjectSubclass<imp::AdminSettingsPage>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for AdminSettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl AdminSettingsPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}
