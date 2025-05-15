use gio::ListModel;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::data_gobject::DataGObject;
use crate::prelude::*;
use crate::settings_gobject::SettingsGObject;

//+list of supported resolutions/modes ?

mod imp {
    use glib::subclass::Signal;
    use glib::Binding;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate, DropDown};
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use crate::data_gobject::DataGObject;
    use crate::prelude::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/language_region_settings_page.ui")]
    pub struct LanguageRegionSettingsPage {
        #[template_child]
        pub language_switch: TemplateChild<DropDown>,

        #[template_child]
        pub timezone_switch: TemplateChild<DropDown>,

        // Vector holding the bindings to properties of `Object`
        pub bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LanguageRegionSettingsPage {
        const NAME: &'static str = "LanguageRegionSettingsPage";
        type Type = super::LanguageRegionSettingsPage;
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
    impl LanguageRegionSettingsPage {
        #[template_callback]
        fn on_reset_clicked(&self) {
            debug!("Reset to defaults!");
            self.obj().emit_by_name::<()>("locale-default", &[]);
        }
        #[template_callback]
        fn on_apply_clicked(&self) {
            let locale = self
                .language_switch
                .selected_obj()
                .as_ref()
                .map_or_else(|| "C".into(), DataGObject::name);
            let timezone = self
                .timezone_switch
                .selected_obj()
                .as_ref()
                .map_or_else(|| "UTC".into(), DataGObject::name);
            debug!("Language and timezone changed! {locale}, {timezone}");
            self.obj()
                .emit_by_name::<()>("locale-timezone-changed", &[&locale, &timezone]);
        }
    } //end #[gtk::template_callbacks]

    impl ObjectImpl for LanguageRegionSettingsPage {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("locale-timezone-changed")
                        .param_types([String::static_type(), String::static_type()])
                        .build(),
                    Signal::builder("locale-default").build(),
                ]
            })
        }
    }
    impl WidgetImpl for LanguageRegionSettingsPage {}
    impl BoxImpl for LanguageRegionSettingsPage {}
}

glib::wrapper! {
pub struct LanguageRegionSettingsPage(ObjectSubclass<imp::LanguageRegionSettingsPage>)
    @extends gtk::Widget, gtk::Box;
}

impl Default for LanguageRegionSettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageRegionSettingsPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
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

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_locale_model(&self, model: impl IsA<ListModel>, selected: Option<usize>) {
        self.imp().language_switch.set_model(Some(&model));
        if let Some(idx) = selected {
            self.imp().language_switch.set_selected(idx as u32);
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn locale_select_find<F: FnMut(&DataGObject) -> bool>(&self, mut filter: F) {
        if let Some(index) = self.imp().language_switch.model().and_then(|m| {
            m.wrap()
                .iter()
                .enumerate()
                .find_map(move |(idx, item)| filter(&item).then_some(idx))
        }) {
            self.imp().language_switch.set_selected(index as u32);
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_timezone_model(&self, model: impl IsA<ListModel>, selected: Option<usize>) {
        self.imp().timezone_switch.set_model(Some(&model));
        if let Some(idx) = selected {
            self.imp().timezone_switch.set_selected(idx as u32);
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn timezone_select_find<F: FnMut(&DataGObject) -> bool>(&self, mut filter: F) {
        if let Some(index) = self.imp().timezone_switch.model().and_then(|m| {
            m.wrap()
                .iter()
                .enumerate()
                .find_map(move |(idx, item)| filter(&item).then_some(idx))
        }) {
            self.imp().timezone_switch.set_selected(index as u32);
        }
    }
}
