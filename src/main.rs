
mod application;
mod window;
mod connection_config;
mod vm_row;
mod vm_row_2;
mod vm_gobject;
mod vm_settings;
mod audio_settings;
mod settings;
mod settings_gobject;
mod data_provider;
mod security_icon;
mod info_settings_page;
mod security_settings_page;
mod wifi_settings_page;
mod mouse_settings_page;
mod vm_control_action;
mod trust_level;
mod add_network_popup;
mod settings_action;

use self::application::ControlPanelGuiApplication;
use self::window::ControlPanelGuiWindow;

use gtk::{gio, glib};
use gtk::prelude::*;

fn main() -> glib::ExitCode {
    //std::env::set_var("RUST_BACKTRACE", "full");

    // Load resources
    gio::resources_register_include!("control_panel_gui.gresource")
        .expect("Failed to register resources.");

    // Create a new GtkApplication. The application manages our main loop,
    // application windows, integration with the window manager/compositor, and
    // desktop features such as file opening and single-instance applications.
    let app = ControlPanelGuiApplication::new("org.gnome.controlpanelgui", &gio::ApplicationFlags::empty());

    // Run the application. This function will block until the application
    // exits. Upon return, we have our exit code to return to the shell. (This
    // is the code you see when you do `echo $?` after running a command in a
    // terminal.
    app.run()
}
