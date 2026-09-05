use godot::init::InitStage;
use std::ffi::c_int;
use std::fs::OpenOptions;
use std::io::Write;

unsafe extern "C" {
    fn __llvm_profile_dump() -> c_int;
}

pub(crate) fn dump(stage: InitStage) {
    if stage != InitStage::Scene {
        return;
    }
    if let Err(error) = dump_scene() {
        eprintln!("coverage flush failed: {error}");
    }
}

fn dump_scene() -> Result<(), String> {
    let status = unsafe { __llvm_profile_dump() };
    let path = std::env::var_os("ITEST_COVERAGE_FLUSH_PATH")
        .ok_or_else(|| "ITEST_COVERAGE_FLUSH_PATH is unset".to_string())?;
    let mut sentinel = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("could not create {path:?}: {error}"))?;
    write!(
        sentinel,
        "{{\"schema_version\":1,\"pid\":{},\"stage\":\"scene\",\"status\":{status}}}\n",
        std::process::id()
    )
    .map_err(|error| format!("could not write {path:?}: {error}"))?;
    sentinel
        .sync_all()
        .map_err(|error| format!("could not sync {path:?}: {error}"))
}
