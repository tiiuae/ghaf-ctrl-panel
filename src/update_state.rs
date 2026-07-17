use gtk::glib::{self, Object};

#[derive(Debug, Clone, PartialEq, Default, glib::Boxed)]
#[boxed_type(name = "UpdateActivity")]
pub enum UpdateActivity {
    #[default]
    Idle,
    Checking,
    Checked,
    Downloading {
        progress: f64,
    },
    Downloaded,
    Installing {
        progress: f64,
    },
    Installed,
}

mod imp {
    use gtk::glib::{self, Properties};
    use gtk::prelude::ObjectExt;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    use super::UpdateActivity;

    #[derive(Default)]
    pub struct UpdateStateData {
        pub current_version: String,
        pub available_version: String,
        pub changelog: String,
        pub download_size_bytes: u64,
        pub activity: UpdateActivity,
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::UpdateState)]
    pub struct UpdateState {
        #[property(name = "current-version", get, set, type = String, member = current_version)]
        #[property(name = "available-version", get, set, type = String, member = available_version)]
        #[property(name = "changelog", get, set, type = String, member = changelog)]
        #[property(name = "download-size-bytes", get, set, type = u64, member = download_size_bytes)]
        #[property(name = "activity", get, set, type = UpdateActivity, member = activity)]
        pub data: RefCell<UpdateStateData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UpdateState {
        const NAME: &'static str = "UpdateState";
        type Type = super::UpdateState;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for UpdateState {}
}

glib::wrapper! {
    pub struct UpdateState(ObjectSubclass<imp::UpdateState>);
}

impl Default for UpdateState {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl UpdateState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_checked(&self) -> bool {
        !matches!(
            self.activity(),
            UpdateActivity::Idle | UpdateActivity::Checking
        )
    }
}
