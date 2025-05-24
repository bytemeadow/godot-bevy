# CI/CD Documentation

This repository uses GitHub Actions to automatically build and test the Rust projects and Godot game for multiple platforms.

## Workflow Overview

The CI workflow (`.github/workflows/ci.yml`) consists of three main jobs:

### 1. Rust Build Job
- **Platforms**: Linux (Ubuntu), macOS, Windows
- **Actions**:
  - Installs Rust toolchain with appropriate targets
  - Runs code formatting checks (`cargo fmt`)
  - Runs linting checks (`cargo clippy`)
  - Runs unit tests (`cargo test`)
  - Builds workspace in debug and release modes
  - Builds the example project specifically
  - Uploads compiled libraries as artifacts

### 2. Godot Build Job
- **Platforms**: Linux (Ubuntu), macOS, Windows
- **Dependencies**: Requires successful Rust build
- **Actions**:
  - Downloads Rust artifacts from previous job
  - Downloads and caches Godot engine binaries
  - Downloads Godot export templates
  - Imports the Godot project
  - Exports the game for each platform
  - Uploads exported games as artifacts

### 3. Release Job
- **Trigger**: Only on tagged releases (tags starting with `v`)
- **Actions**:
  - Downloads all artifacts
  - Creates a GitHub release with all built binaries

## Supported Platforms

| Platform | Rust Target | Godot Export | Library Extension |
|----------|-------------|--------------|------------------|
| Linux    | x86_64-unknown-linux-gnu | Linux/X11 | `.so` |
| macOS    | x86_64-apple-darwin | macOS | `.dylib` |
| Windows  | x86_64-pc-windows-msvc | Windows Desktop | `.dll` |

## Project Structure

The workflow expects the following structure:
```
├── Cargo.toml (workspace)
├── examples/
│   └── dodge-the-creeps-2d/
│       ├── rust/ (Rust GDExtension project)
│       └── godot/ (Godot game project)
└── .github/
    └── workflows/
        └── ci.yml
```

## Artifacts

The CI produces the following artifacts:
- `rust-libs-{os}`: Compiled Rust libraries for each platform
- `godot-export-{os}`: Exported Godot games for each platform

## Configuration Files

### Rust Configuration
- `examples/dodge-the-creeps-2d/rust/Cargo.toml`: Rust project dependencies
- `Cargo.toml`: Workspace configuration

### Godot Configuration
- `examples/dodge-the-creeps-2d/godot/rust.gdextension`: GDExtension library configuration for all platforms
- `examples/dodge-the-creeps-2d/godot/export_presets.cfg`: Export presets for Linux, Windows, and macOS
- `examples/dodge-the-creeps-2d/godot/project.godot`: Main Godot project file

## Triggering Builds

### Automatic Triggers
- Push to `main` or `develop` branches
- Pull requests to `main` or `develop` branches

### Manual Triggers
- Can be triggered manually from the GitHub Actions tab

### Release Builds
- Push a tag starting with `v` (e.g., `v1.0.0`) to trigger a release build
- Creates a GitHub release with all platform binaries

## Caching

The workflow uses caching to speed up builds:
- **Cargo cache**: Caches Rust dependencies and build artifacts
- **Godot cache**: Caches downloaded Godot binaries

## Troubleshooting

### Common Issues

1. **Missing dependencies**: Ensure all required Rust dependencies are specified in `Cargo.toml`
2. **Godot export failure**: Check that export presets are properly configured
3. **Platform-specific issues**: Review platform-specific build steps in the workflow

### Debugging

- Check the Actions tab in GitHub for detailed build logs
- Each job runs independently, so you can identify which component is failing
- Artifacts are uploaded even on partial failures for debugging

## Local Development

To test locally before pushing:

```bash
# Test Rust build
cargo build --release
cargo test

# Test formatting and linting
cargo fmt --check
cargo clippy -- -D warnings

# Build specific example
cargo build --release --manifest-path examples/dodge-the-creeps-2d/rust/Cargo.toml
```

For Godot testing, you'll need to:
1. Install Godot 4.4
2. Import the project in `examples/dodge-the-creeps-2d/godot/`
3. Test exports manually 