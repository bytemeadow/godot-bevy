fn main() {
    let result = cargo_godot_lib::GodotRunner::create(
        env!("CARGO_PKG_NAME"),
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../godot"),
    )
    .and_then(|runner| runner.execute());
    if let Err(e) = result {
        eprintln!("{e}");
    }
}
