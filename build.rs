use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    glib_build_tools::compile_resources(
        &["src/"],
        "src/control_panel_gui.gresource.xml",
        "control_panel_gui.gresource",
    );

    let proto = "proto/admin.proto";

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("admin_descriptor.bin"))
        .compile(&["proto/admin.proto"], &["admin"])
        .unwrap();

    println!("OK!!!");
    Ok(())
}