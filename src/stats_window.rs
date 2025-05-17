use gtk::glib;

mod imp {
    use std::cell::RefCell;

    use glib::Properties;
    use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate};

    use crate::application::ControlPanelGuiApplication;
    use crate::plot::Plot;
    use crate::serie::Serie;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/org/gnome/controlpanelgui/ui/stats_window.ui")]
    #[properties(wrapper_type = super::StatsWindow)]
    pub struct StatsWindow {
        #[property(get, set, construct_only)]
        vm: RefCell<String>,

        #[template_child]
        cpu_plot: TemplateChild<Plot>,
        #[template_child]
        cpu_user_serie: TemplateChild<Serie>,
        #[template_child]
        cpu_sys_serie: TemplateChild<Serie>,

        #[template_child]
        memory_plot: TemplateChild<Plot>,
        #[template_child]
        mem_used_serie: TemplateChild<Serie>,
        #[template_child]
        mem_needed_serie: TemplateChild<Serie>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StatsWindow {
        const NAME: &'static str = "StatsWindow";
        type Type = super::StatsWindow;
        type ParentType = gtk::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.set_css_name("statswindow");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for StatsWindow {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj()
                .set_title(Some(&format!("Monitoring VM {vm}", vm = self.vm.borrow())));
            self.cpu_plot.set_view(None, None, Some(0.), Some(1.));
            self.cpu_plot
                .set_label_format(|f| format!("{pct:.0}%", pct = f * 100.));

            self.memory_plot
                .set_label_format(|f| format!("{mb:.0} MB", mb = f / 1_048_576.));

            self.start();
        }
    }

    impl WidgetImpl for StatsWindow {}
    impl WindowImpl for StatsWindow {}

    impl StatsWindow {
        fn start(&self) {
            self.obj().connect_application_notify(|win| {
                let app = win.imp().get_app();
                let mut i = 0.;

                glib::spawn_future_local(glib::clone!(
                    #[strong(rename_to = win)]
                    win.downgrade(),
                    async move {
                        loop {
                            let Some(win) = win.upgrade() else {
                                break;
                            };
                            let imp = win.imp();
                            if let Ok(stats) = app.get_stats(imp.vm.borrow().clone()).await {
                                if let Some(process) = stats.process {
                                    imp.cpu_user_serie.push(
                                        i,
                                        process.user_cycles as f32 / process.total_cycles as f32,
                                    );
                                    imp.cpu_sys_serie.push(
                                        i,
                                        (process.user_cycles + process.sys_cycles) as f32
                                            / process.total_cycles as f32,
                                    );
                                }
                                if let Some(memory) = stats.memory {
                                    imp.memory_plot.set_view(
                                        None,
                                        None,
                                        Some(0.),
                                        Some(memory.total as f32),
                                    );
                                    imp.mem_needed_serie
                                        .push(i, (memory.total - memory.available) as f32);
                                    imp.mem_used_serie
                                        .push(i, (memory.total - memory.free) as f32);
                                }
                                i += 1.;
                            }
                            glib::timeout_future_seconds(1).await;
                        }
                    }
                ));
            });
        }

        #[inline]
        fn get_app(&self) -> ControlPanelGuiApplication {
            let binding = self.obj().application().expect("Failed to get application");
            binding
                .downcast()
                .expect("ControlPanelGuiApplication is expected!")
        }
    }
}

glib::wrapper! {
    pub struct StatsWindow(ObjectSubclass<imp::StatsWindow>)
        @extends gtk::Widget, gtk::Window,
        @implements gtk::Buildable, gtk::Root;
}

impl Default for StatsWindow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl StatsWindow {
    pub fn new(vm: impl AsRef<str>) -> Self {
        glib::Object::builder().property("vm", vm.as_ref()).build()
    }
}
