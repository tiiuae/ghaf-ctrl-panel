fn main() {
    glib_build_tools::compile_resources(
        &["src/"],
        "src/control_panel_gui.gresource.xml",
        "control_panel_gui.gresource",
    );

    glib_build_tools::compile_resources(
        &["src/"],
        "src/bugreport.gresource.xml",
        "bugreport.gresource",
    );
}
