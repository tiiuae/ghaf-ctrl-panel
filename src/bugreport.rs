mod bug_report_settings_page;
mod github;

pub mod prelude {
    pub use log::{debug, error, info, warn};
}

use clap::Parser;
use gtk::{CssProvider, gdk, gio, glib, prelude::*};

use crate::prelude::*;

mod imp {
    use adw::subclass::prelude::*;
    use gtk::glib;

    use crate::bug_report_settings_page::BugReportSettingsPage;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/ae/tii/ghaf/bugreport/ui/bugreport_window.ui")]
    pub struct BugReportWindow {
        #[template_child]
        pub bug_report_page: TemplateChild<BugReportSettingsPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BugReportWindow {
        const NAME: &'static str = "BugReportWindow";
        type Type = super::BugReportWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BugReportWindow {}
    impl WidgetImpl for BugReportWindow {}
    impl WindowImpl for BugReportWindow {}
    impl ApplicationWindowImpl for BugReportWindow {}
    impl AdwApplicationWindowImpl for BugReportWindow {}

    #[gtk::template_callbacks]
    impl BugReportWindow {}
}

glib::wrapper! {
    pub struct BugReportWindow(ObjectSubclass<imp::BugReportWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
            gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[derive(clap::ValueEnum, Default, Debug, Clone, Copy, PartialEq)]
pub enum LogOutput {
    #[default]
    Syslog,
    Stdout,
}

#[derive(Parser, Debug)]
#[command(name = "bugreport")]
#[command(about = "Ghaf Bug Reporter", long_about = None)]
struct Args {
    /// Log severity
    #[arg(long, default_value_t = log::Level::Info)]
    pub log_level: log::Level,

    /// Log output
    #[arg(long, value_enum, default_value_t)]
    pub log_output: LogOutput,
}

fn main() /*-> glib::ExitCode*/
{
    //std::env::set_var("RUST_BACKTRACE", "full");
    // Parse the command-line arguments
    let args = Args::parse();
    initialize_logger(&args);

    // Load resources
    gio::resources_register_include!("bugreport.gresource").expect("Failed to register resources.");

    let app = adw::Application::new(
        Some("ae.tii.ghaf./bugreporter"),
        gio::ApplicationFlags::default(),
    );

    gtk::init().expect("Failed");
    load_css();

    app.connect_activate(|app| {
        app.windows()
            .into_iter()
            .find_map(|w| w.downcast::<BugReportWindow>().ok())
            .unwrap_or_else(|| {
                let window = glib::Object::builder().build();
                app.add_window(&window);
                window
            })
            .present();
    });

    app.run_with_args(&[""; 0]);
}

fn initialize_logger(args: &Args) {
    // Initialize env_logger
    let log_level = args.log_level.to_level_filter();
    match args.log_output {
        LogOutput::Stdout => {
            // You can set the level in code here
            env_logger::Builder::new()
                .filter_level(log_level) // Set to Debug level in code
                .init();
            debug!("Logging to stdout");
        }
        LogOutput::Syslog => {
            debug!("Logging to syslog");
            let formatter = syslog::Formatter3164 {
                process: "ghaf-ctrl-panel".into(),
                ..Default::default()
            };
            let logger = match syslog::unix(formatter) {
                Err(e) => {
                    error!("Failed to connect to syslog: {e}");
                    return;
                }
                Ok(logger) => logger,
            };
            log::set_boxed_logger(Box::new(syslog::BasicLogger::new(logger)))
                .expect("Failed to set logger");
            log::set_max_level(log_level);
        }
    }

    debug!("Logger initialized");
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_resource("/ae/tii/ghaf/bugreport/styles/style.css");

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
