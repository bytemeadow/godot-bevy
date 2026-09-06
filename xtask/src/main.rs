// Generate descriptors for CI exports without launching Godot.
use cargo_godot_lib::gdextension_config::GdExtensionConfig;
use cargo_metadata::MetadataCommand;
use std::{env, error::Error, path::PathBuf, process};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [command, flag, manifest] = args.as_slice() else {
        return Err("usage: xtask gdextension --manifest-path <Cargo.toml>".into());
    };
    if command != "gdextension" || flag != "--manifest-path" {
        return Err("usage: xtask gdextension --manifest-path <Cargo.toml>".into());
    }

    let manifest = PathBuf::from(manifest).canonicalize()?;
    let metadata = MetadataCommand::new()
        .manifest_path(&manifest)
        .no_deps()
        .exec()?;
    let package = metadata
        .packages
        .iter()
        .find(|package| package.manifest_path.as_std_path() == manifest)
        .ok_or("manifest does not identify a workspace package")?;
    let project = manifest
        .parent()
        .ok_or("manifest has no parent directory")?
        .join("../godot");
    let config = GdExtensionConfig::start(
        package.name.as_ref(),
        &project,
        metadata.target_directory.as_std_path(),
    )
    .build()?;
    config.write()?;
    println!("{}", config.full_config_path().display());
    Ok(())
}
