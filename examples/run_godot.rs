#[cfg(not(feature = "itest"))]
fn main() {
    if std::env::var_os("GODOT_BEVY_CAPTURE").is_some() {
        let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("example directory");
        let driver = example.join("../../itest/capture/capture.py");
        let status = std::process::Command::new("python3")
            .arg(&driver)
            .arg("--example")
            .arg(example.file_name().expect("example name"))
            .arg("--check-library")
            .status();
        match status {
            Ok(status) if status.success() => {}
            Ok(status) => std::process::exit(status.code().unwrap_or(2)),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
        let mut command = std::process::Command::new("python3");
        command
            .arg(driver)
            .arg("--example")
            .arg(example.file_name().expect("example name"));
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            eprintln!("{}", command.exec());
            std::process::exit(2);
        }
        #[cfg(not(unix))]
        std::process::exit(match command.status() {
            Ok(status) => status.code().unwrap_or(1),
            Err(error) => {
                eprintln!("{error}");
                2
            }
        });
    }

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
    unsafe { std::env::set_var("GODOT_BEVY_ITEST", "1") };

    // Godot always loads the `.debug` entry, so point both entries at the profile
    // this binary was built with; otherwise `cargo run --release` loads nothing.
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let runner = cargo_godot_lib::GodotRunner::create(
        env!("CARGO_PKG_NAME"),
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../godot"),
    )
    .gdextension_config(move |config| {
        config
            .debug_target(Some(profile.to_string()))
            .release_target(Some(profile.to_string()))
    });

    let runner = runner.godot_cli_arguments(vec![
        "--headless",
        "--fixed-fps",
        "60",
        "--scene",
        "res://addons/godot-bevy/test/TestRunner.tscn",
        "--quit-after",
        "10000",
    ]);

    // Discard a leftover result from an earlier crashed run.
    godot_bevy_test::exit_code::read_and_cleanup_exit_code();

    // Godot can die on a signal at shutdown with a GDExtension loaded, after the tests
    // have already written their result, so the exit file is authoritative.
    if let Err(e) = runner.execute() {
        eprintln!("{e}");
    }

    std::process::exit(godot_bevy_test::exit_code::read_and_cleanup_exit_code().unwrap_or(1));
}
