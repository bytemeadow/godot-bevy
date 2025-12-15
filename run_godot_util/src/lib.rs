use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use which::{which, which_in_global};

pub fn run_godot(
    godot_project_path: &Path,
    crate_name: &str,
    gdextension_file_name: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("+-------- Launching Godot project");

    let absolute_godot_project_path = godot_project_path.canonicalize().map_err(|e| {
        format!(
            "Failed to canonicalize Godot project path `{}`, reason: {}",
            godot_project_path.display(),
            e
        )
    })?;
    println!(
        "| Working directory: {}",
        absolute_godot_project_path.display()
    );

    // Detect build profile
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    println!("| Running with Rust build profile: {profile}");

    generate_gdextension_file(
        &absolute_godot_project_path,
        gdextension_file_name,
        profile,
        &get_cargo_target_dir()?,
        crate_name,
    )?;

    let godot_binary_path = godot_binary_path()?;
    println!("| Godot binary path: {}", godot_binary_path.display());
    println!("+--------");

    run_godot_import_if_needed(&absolute_godot_project_path)
        .map_err(|e| format!("Failed to run Godot import: {}", e))?;

    let mut child = Command::new(godot_binary_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(&absolute_godot_project_path)
        .arg("--debug") // Launch Godot with local stdout debugger
        .spawn()?;

    let status = child.wait()?;

    if !status.success() {
        return if let Some(code) = status.code() {
            Err(format!("Godot process failed with exit code {}", code).into())
        } else {
            Err("Godot process failed with unknown exit code".into())
        };
    }

    Ok(())
}

pub fn run_godot_import_if_needed(
    godot_project_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if !godot_project_path.join(".godot").exists() {
        return run_godot_import(godot_project_path);
    }
    Ok(())
}

pub fn run_godot_import(godot_project_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("+-------- Running Godot import");
    let godot_binary_path = godot_binary_path()?;
    let mut child = Command::new(godot_binary_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(godot_project_path)
        .arg("--import") // Launch Godot with local stdout debugger
        .arg("--headless") // Launch Godot with local stdout debugger
        .spawn()?;

    let status = child.wait()?;
    println!("+-------- Import complete");

    if !status.success() {
        let message = format!(
            "Godot import process failed with exit code `{}`.\n\
            Possible cause: Known bug in Godot 4.5.1: \"Headless import of project with GDExtensions crashes\"\n\
            See: https://github.com/godotengine/godot/issues/111645\n\
            Try re-running if `.godot` folder was generated successfully.",
            status
                .code()
                .map(|e| e.to_string())
                .unwrap_or("unknown".to_string())
        );
        return Err(message.into());
    }

    Ok(())
}

pub fn get_cargo_target_dir() -> Result<PathBuf, std::io::Error> {
    // Run `cargo metadata --format-version 1 --no-deps` and parse the JSON output using serde_json
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "`cargo metadata` failed with status {}",
            output.status
        )));
    }

    // Parse JSON from stdout
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|e| {
        std::io::Error::other(format!("failed to parse `cargo metadata` JSON: {}", e))
    })?;

    if let Some(dir) = v.get("target_directory").and_then(|t| t.as_str()) {
        Ok(PathBuf::from(dir))
    } else {
        Err(std::io::Error::other(
            "target_directory not found in `cargo metadata` output",
        ))
    }
}

pub fn generate_gdextension_file(
    godot_project_path: &Path,
    gdextension_file_name: Option<&str>,
    build_profile: &str,
    cargo_target_path: &Path,
    crate_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let absolute_project_path = godot_project_path.canonicalize().map_err(|e| {
        format!(
            "Failed to canonicalize godot project path `{}`, reason: {}",
            godot_project_path.display(),
            e
        )
    })?;
    let gdextension_file_path =
        absolute_project_path.join(gdextension_file_name.unwrap_or("rust.gdextension"));
    let libs_path = cargo_target_path.join(build_profile).canonicalize()?;
    let relative_libs_path = pathdiff::diff_paths(&libs_path, godot_project_path)
        .ok_or("Failed to compute relative library path")?;

    println!("| Generating gdextension file...");
    println!("|   destination: {}", gdextension_file_path.display());
    println!("|   profile: {}", build_profile);
    println!("|   cargo libs directory: {}", libs_path.display());
    println!(
        "|   relative cargo libs directory: {}",
        relative_libs_path.display()
    );

    let file = format!(
        r#"
[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.4
reloadable = false

[libraries]
linux.debug        = "res://{0}/lib{1}.so"
linux.release      = "res://{0}/lib{1}.so"
macos.debug        = "res://{0}/lib{1}.dylib"
macos.release      = "res://{0}/lib{1}.dylib"
windows.debug      = "res://{0}/{1}.dll"
windows.release    = "res://{0}/{1}.dll"

[icons]
"#,
        relative_libs_path
            .to_str()
            .expect("target_dir is not valid utf-8"),
        crate_name.replace("-", "_"),
    );
    let file = file.trim_start();
    fs::write(gdextension_file_path, file).expect("Failed to write gdextension file");
    Ok(())
}

fn godot_binary_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // if let Some(path) = std::env::var_os("PATH") {
    //     println!("PATH = {:?}", path);
    //     for p in std::env::split_paths(&path) {
    //         println!("  | {}", p.display());
    //     }
    // }

    if let Ok(godot_binary_path) = std::env::var("godot") {
        return Ok(PathBuf::from(godot_binary_path));
    }

    if let Ok(godot_binary_path) = which("godot") {
        return Ok(godot_binary_path);
    }

    // Search in some reasonable locations across linux and osx for godot.
    // Windows is trickier, as I believe the binary name contains the version
    // of godot, e.g., C:\\Program Files\\Godot\\Godot_v3.4.2-stable_win64.exe
    let godot_search_paths = "/usr/local/bin:/usr/bin:/bin:/Applications/Godot.app/Contents/MacOS";

    if let Some(godot_binary_path) = which_in_global("godot", Some(godot_search_paths))
        .ok()
        .and_then(|it| it.into_iter().next())
    {
        return Ok(godot_binary_path);
    }

    Err("Couldn't find the godot binary in your environment's path or in default search locations ({godot_search_paths:?})".into())
}
