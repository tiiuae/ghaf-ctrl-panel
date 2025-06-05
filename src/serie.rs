use gtk::{glib, subclass::prelude::*};

mod imp {
    use glib::{subclass::Signal, Properties};
    use gtk::{gdk, glib, prelude::*, subclass::prelude::*};
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::sync::OnceLock;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Serie)]
    pub struct Serie {
        #[property(get, set = Serie::set_window)]
        window: Cell<u32>,

        #[property(get, set = Serie::set_color)]
        color: RefCell<String>,

        points: RefCell<VecDeque<(f32, f32)>>,

        #[property(get)]
        actual_color: Cell<Option<gdk::RGBA>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Serie {
        const NAME: &'static str = "Serie";
        type Type = super::Serie;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Serie {
        fn constructed(&self) {
            self.actual_color.set(self.color.borrow().parse().ok());
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("changed").build()])
        }
    }

    impl Serie {
        #[allow(clippy::semicolon_if_nothing_returned)]
        pub fn push(&self, x: f32, y: f32) {
            let mut pts = self.points.borrow_mut();
            pts.push_back((x, y));
            pts.drain(..(self.window.get() as usize));
            self.obj().emit_by_name("changed", &[])
        }

        pub fn get(&self, idx: u32) -> Option<(f32, f32)> {
            self.points.borrow().get(idx as usize).copied()
        }

        #[allow(clippy::semicolon_if_nothing_returned)]
        fn set_window(&self, window: u32) {
            self.window.set(window);
            let mut pts = self.points.borrow_mut();
            if pts.len() > window as usize {
                pts.drain(..(window as usize));
                drop(pts);
                self.obj().emit_by_name("changed", &[])
            }
        }

        fn set_color(&self, color: &str) {
            color.clone_into(&mut self.color.borrow_mut());
            self.actual_color.set(color.parse().ok());
        }
    }
}

glib::wrapper! {
    pub struct Serie(ObjectSubclass<imp::Serie>)
        @implements gtk::Buildable;
}

impl Default for Serie {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Serie {
    pub fn push(&self, x: f32, y: f32) {
        self.imp().push(x, y);
    }

    pub fn values(&self) -> impl Iterator<Item = (f32, f32)> + use<'_> {
        (0..).map_while(|i| self.imp().get(i))
    }
}
