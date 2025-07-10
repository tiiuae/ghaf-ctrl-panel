mod add_network_popup;
mod admin_settings_page;
mod application;
mod audio_control;
mod audio_device_gobject;
mod audio_settings;
mod bug_report_settings_page;
mod connection_config;
mod control_action;
mod data_gobject;
mod error_popup;
mod github;
mod info_settings_page;
mod keyboard_settings_page;
mod language_region_notify_popup;
mod language_region_settings_page;
mod locale_provider;
mod plot;
mod prelude;
mod security_icon;
mod security_settings_page;
mod serie;
mod service_gobject;
mod service_model;
mod service_row;
mod service_settings;
mod settings;
mod settings_action;
mod settings_gobject;
mod trust_level;
mod typed_list_store;
mod vm_row;
mod vm_status;
mod volume_widget;
mod wifi_settings_page;
mod window;
mod wireguard_vms;

use self::application::ControlPanelGuiApplication;
use self::window::ControlPanelGuiWindow;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

use givc_client::endpoint::TlsConfig;
use gtk::prelude::*;
use gtk::{gio, glib};
use syslog::{BasicLogger, Formatter3164};

use crate::wireguard_vms::initialize_wvm_list;
use env_logger::Builder;
use prelude::*;

const ADMIN_SERVICE_ADDR: &str = "192.168.101.10";
const ADMIN_SERVICE_PORT: u16 = 9001;

#[derive(ValueEnum, Default, Debug, Clone, Copy, PartialEq)]
pub enum LogOutput {
    #[default]
    Syslog,
    Stdout,
}

#[derive(Parser, Debug)]
#[command(name = "ctrl-panel")]
#[command(about = "Ghaf Control Panel", long_about = None)]
struct Args {
    #[arg(long)]
    addr: Option<String>,
    #[arg(long)]
    port: Option<u16>,

    #[arg(long, env = "NAME", default_value = "admin-vm")]
    name: String, // for TLS service name

    #[arg(long, env = "CA_CERT", default_value = "/run/givc/ca-cert.pem")]
    cacert: Option<PathBuf>,

    #[arg(long, env = "HOST_CERT", default_value = "/run/givc/cert.pem")]
    cert: Option<PathBuf>,

    #[arg(long, env = "HOST_KEY", default_value = "/run/givc/key.pem")]
    key: Option<PathBuf>,

    #[arg(long, default_value = "/etc/ctrl-panel/wireguard-gui-vms.txt")]
    wireguardlist: Option<PathBuf>,

    #[arg(long, default_value_t)]
    notls: bool,

    /// Log severity
    #[arg(long, default_value_t = log::Level::Info)]
    pub log_level: log::Level,

    /// Log output
    #[arg(long, value_enum, default_value_t)]
    pub log_output: LogOutput,
}

fn initialize_logger(args: &Args) {
    // Initialize env_logger
    let log_level = args.log_level.to_level_filter();
    match args.log_output {
        LogOutput::Stdout => {
            // You can set the level in code here
            Builder::new()
                .filter_level(log_level) // Set to Debug level in code
                .init();
            debug!("Logging to stdout");
        }
        LogOutput::Syslog => {
            debug!("Logging to syslog");
            let formatter = Formatter3164 {
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
            log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
                .expect("Failed to set logger");
            log::set_max_level(log_level);
        }
    }

    debug!("Logger initialized");
}

fn main() /*-> glib::ExitCode*/
{
    //std::env::set_var("RUST_BACKTRACE", "full");
    // Parse the command-line arguments
    let args = Args::parse();
    initialize_logger(&args);

    let addr = if let Some(addr) = args.addr {
        addr
    } else {
        String::from(ADMIN_SERVICE_ADDR)
    };

    let port = if let Some(port) = args.port {
        port
    } else {
        ADMIN_SERVICE_PORT
    };

    let tls_info = if args.notls {
        None
    } else {
        Some((
            args.name.clone(),
            TlsConfig {
                ca_cert_file_path: args.cacert.expect("cacert is required"),
                cert_file_path: args.cert.expect("cert is required"),
                key_file_path: args.key.expect("key is required"),
                tls_name: Some(args.name),
            },
        ))
    };

    //read file with wireguard VMs
    initialize_wvm_list(&args.wireguardlist.expect("wireguard vm list file required"));

    // Load resources
    gio::resources_register_include!("control_panel_gui.gresource")
        .expect("Failed to register resources.");

    // Create a new GtkApplication. The application manages our main loop,
    // application windows, integration with the window manager/compositor, and
    // desktop features such as file opening and single-instance applications.
    let app = ControlPanelGuiApplication::new(
        "org.gnome.controlpanelgui",
        gio::ApplicationFlags::empty(),
        addr,
        port,
        tls_info,
    );

    // Run the application. This function will block until the application
    // exits. Upon return, we have our exit code to return to the shell. (This
    // is the code you see when you do `echo $?` after running a command in a
    // terminal.
    //put empty array as args, cause we need our ones to be processed
    app.run_with_args(&[""; 0]);
}
