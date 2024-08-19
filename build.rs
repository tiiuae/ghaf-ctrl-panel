use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    glib_build_tools::compile_resources(
        &["src/"],
        "src/control_panel_gui.gresource.xml",
        "control_panel_gui.gresource",
    );

    Ok(())
}