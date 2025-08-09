use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio, exit},
};

use which::{which, which_in_global};

fn main() -> Result<(), std::io::Error> {
    let run_dir = format!("{}/../godot", env!("CARGO_MANIFEST_DIR"));
    println!("run: {run_dir:?}");

    // Detect build profile
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    println!("Running with Rust build profile: {profile}");

    // Update gdextension file if running in release mode
    let gdextension_path = Path::new(&run_dir).join("rust.gdextension");
    let original_content = if profile == "release" && gdextension_path.exists() {
        Some(update_gdextension_for_release(&gdextension_path)?)
    } else {
        None
    };

    let mut child = Command::new(godot_binary_path())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(&run_dir)
        .spawn()?;

    let status = child.wait()?;

    // Restore original gdextension content if we modified it
    if let Some(content) = original_content {
        fs::write(&gdextension_path, content)?;
        println!("Restored original gdextension file");
    }

    match status.code() {
        Some(code) => exit(code),
        None => {
            println!("Process terminated by signal");
            exit(255);
        }
    }
}

fn update_gdextension_for_release(path: &Path) -> Result<String, std::io::Error> {
    let original_content = fs::read_to_string(path)?;
    let mut lines: Vec<String> = original_content.lines().map(|s| s.to_string()).collect();

    println!("Updating gdextension to use release builds for all configurations...");

    // Update debug entries to point to release builds
    for line in &mut lines {
        if line.contains(".debug.") && (line.contains("/debug/") || line.contains("\\debug\\")) {
            *line = line
                .replace("/debug/", "/release/")
                .replace("\\debug\\", "\\release\\");
        }
    }

    let modified_content = lines.join("\n") + "\n";
    fs::write(path, &modified_content)?;

    Ok(original_content)
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
    // of godot, e.g., C:\\Program Files\\Godot\\Godot_v3.4.2-stable_win64.exe
    let godot_search_paths = "/usr/local/bin:/usr/bin:/bin:/Applications/Godot.app/Contents/MacOS";

    if let Some(godot_binary_path) = which_in_global("godot", Some(godot_search_paths))
        .ok()
        .and_then(|it| it.into_iter().next())
    {
        return godot_binary_path;
    }

    panic!(
        "Couldn't find the godot binary in your environment's path or in default search locations ({godot_search_paths:?})"
    );
}
