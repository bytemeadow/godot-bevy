#[cfg(not(feature = "itest"))]
fn main() {
    let runner = cargo_godot_lib::GodotRunner::create(
        env!("CARGO_PKG_NAME"),
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../godot"),
    );
    if let Err(e) = runner.execute() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

#[cfg(feature = "itest")]
fn main() {
    let runner = cargo_godot_lib::GodotRunner::create(
        env!("CARGO_PKG_NAME"),
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../godot"),
    );

    // Run with `cargo run --features itest` to run integration tests
    #[cfg(feature = "itest")]
    let runner = runner.godot_cli_arguments(vec![
        "--headless",
        "--scene",
        "res://addons/godot-bevy/test/TestRunner.tscn",
        "--quit-after",
        "10000",
    ]);

    if let Err(e) = runner.execute() {
        eprintln!("{e}");
        std::process::exit(1);
    }

    std::process::exit(godot_bevy_test::exit_code::read_and_cleanup_exit_code().unwrap_or(1));
}
