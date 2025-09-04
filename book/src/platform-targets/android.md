# Android

This guide covers building godot-bevy projects for Android devices. Android development requires cross-compilation from your development machine to ARM64 architecture used by most modern Android devices.

## Prerequisites

1. **Android NDK** - The Native Development Kit provides the compilers and tools needed to build native code for Android. Download from [Android NDK Downloads](https://developer.android.com/ndk/downloads)

2. **Rust target** - Install the Android ARM64 compilation target for Rust:
   ```bash
   rustup target add aarch64-linux-android
   ```

## Step 1: Configure Build Environment

The Rust cc build system needs explicit paths to Android NDK compilers for cross-compilation. This tells Rust which Android-specific compilers to use instead of your system's default compilers.

### Option A: Environment Variables
Set these once per terminal session:
```bash
export NDK_HOME="/path/to/your/android/ndk"
export CC="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang"
export CXX="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang++"
export AR="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
```

### Option B: Inline Command
Self-contained approach that doesn't modify your shell environment:
```bash
CC="$NDK_HOME/..." CXX="$NDK_HOME/..." AR="$NDK_HOME/..." cargo build --target aarch64-linux-android
```

## Step 2: Configure Cargo Linker

Create `.cargo/config.toml` in your `rust/` directory. This tells Cargo how to link the compiled code into Android-compatible libraries:

```toml
[target.aarch64-linux-android]
linker = "/path/to/your/android/ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang"
ar = "/path/to/your/android/ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
```

## Step 3: Modify Entry Point

Android requires a specific entry point function name. Change your Bevy app entry point (likely `lib.rs`) from:
```rust
#[bevy_app]
fn build_bevy_app(app: &mut App) {
    // ...
}
```

To:
```rust
#[bevy_app]
#[no_mangle]
fn android_main(app: &mut App) {
    // ...
}
```

The `#[no_mangle]` attribute prevents Rust from changing the function name during compilation, ensuring Android can find the entry point.

## Step 4: Build for Android

Build your Rust library for Android:
```bash
cargo build --target aarch64-linux-android
```
or use the self-contained command from above if you haven't setup the envs.

This creates a shared library file `lib{your_app_name}.so` in `rust/target/aarch64-linux-android/debug/` that Android can load.

## Step 5: Update Godot Configuration

### Update rust.gdextension

Tell Godot where to find your Android library by adding these paths to your `rust.gdextension` file:
```ini
[libraries]
# ... existing entries ...
android.debug.arm64 = "res://rust/target/aarch64-linux-android/debug/lib{your_app_name}.so"
android.release.arm64 = "res://rust/target/aarch64-linux-android/release/lib{your_app_name}.so"
```

### Configure Godot Export

1. Go to **Project → Export...**
2. Select Android preset
3. Under **Architectures**, ensure **arm64-v8a** is selected

> **Note**: The Rust target `aarch64-linux-android` corresponds to Android's `arm64-v8a` architecture

Export your app and deploy to your Android device.
