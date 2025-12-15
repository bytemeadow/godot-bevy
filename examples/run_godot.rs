use run_godot_util::run_godot;
use std::path::PathBuf;
use std::process::exit;

fn main() {
    let godot_project_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../godot");
    if let Err(e) = run_godot(godot_project_path.as_path(), env!("CARGO_PKG_NAME"), None) {
        eprintln!("Error running Godot: {}", e);
        exit(1);
    }
}
