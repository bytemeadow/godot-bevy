//! Cross-platform exit code handling for test runners

use std::path::PathBuf;

/// Get the cross-platform path for exit code file
pub fn exit_code_path() -> PathBuf {
    // Use environment variable if set (for CI customization)
    if let Ok(path) = std::env::var("GODOT_TEST_EXIT_CODE_PATH") {
        return PathBuf::from(path);
    }

    // Cross-platform temp directory
    let mut path = std::env::temp_dir();
    path.push("godot_test_exit_code");
    path
}

/// Write exit code to file for the wrapper script to read
pub fn write_exit_code(code: i32) {
    let path = exit_code_path();
    if let Err(e) = std::fs::write(&path, code.to_string()) {
        eprintln!("Warning: Failed to write exit code to {path:?}: {e}");
    }
}

/// Read and cleanup exit code file (used by test scripts)
pub fn read_and_cleanup_exit_code() -> Option<i32> {
    let path = exit_code_path();
    let code = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok());
    let _ = std::fs::remove_file(&path);
    code
}
