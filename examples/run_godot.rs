use std::{
    path::PathBuf,
    process::{exit, Command, Stdio},
};

use which::{which, which_in_global};

fn main() -> Result<(), std::io::Error> {
    let run_dir = format!("{}/../godot", env!("CARGO_MANIFEST_DIR"));
    println!("run: {run_dir:?}");

    let mut child = Command::new(godot_binary_path())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(&run_dir)
        .spawn()?;

    let status = child.wait()?;
    match status.code() {
        Some(code) => exit(code),
        None => {
            println!("Process terminated by signal");
            exit(255);
        }
    }
}

fn godot_binary_path() -> PathBuf {
    if let Ok(godot_binary_path) = std::env::var("godot") {
        return PathBuf::from(godot_binary_path);
    }

    if let Ok(godot_binary_path) = which("godot") {
        return godot_binary_path;
    }

    // Search in some reasonable locations across linux and osx for godot.
    // Windows is trickier, as I believe the binary name contains the version
    // of godot, e.g., C:\Program Files\Godot\Godot_v3.4.2-stable_win64.exe
    let godot_search_paths = "/usr/local/bin:/usr/bin:/bin:/Applications/Godot.app/Contents/MacOS";

    if let Ok(path_it) = which_in_global("godot", Some(godot_search_paths)) {
        if let Some(godot_binary_path) = path_it.into_iter().next() {
            return godot_binary_path;
        }
    }

    panic!(
        "Couldn't find the godot binary in your environment's path or in default search locations ({godot_search_paths:?})"
    );
}
