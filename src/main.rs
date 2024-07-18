
mod application;
mod window;
mod vm_row;
mod vm_gobject;
mod vm_settings;
mod audio_settings;
mod settings;
mod settings_gobject;
mod data_provider;
mod security_icon;

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
