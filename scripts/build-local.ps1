# Local build script to test CI pipeline locally (PowerShell version)
# This script mimics the CI workflow for local testing on Windows

$ErrorActionPreference = "Stop"

Write-Host "🚀 Starting local build process..." -ForegroundColor Green

function Write-Status {
    param($Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Write-Warning {
    param($Message)
    Write-Host "⚠ $Message" -ForegroundColor Yellow
}

function Write-Error {
    param($Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

# Check if we're in the right directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Error "Error: Please run this script from the project root directory"
    exit 1
}

Write-Host "📦 Building Rust workspace..." -ForegroundColor Cyan

# Format check
Write-Host "🔍 Checking code formatting..." -ForegroundColor Cyan
try {
    cargo fmt --all -- --check
    Write-Status "Code formatting is correct"
} catch {
    Write-Error "Code formatting issues found. Run 'cargo fmt' to fix."
    exit 1
}

# Clippy check
Write-Host "🔍 Running clippy..." -ForegroundColor Cyan
try {
    cargo clippy --all-targets --all-features -- -D warnings
    Write-Status "Clippy checks passed"
} catch {
    Write-Error "Clippy found issues"
    exit 1
}

# Run tests
Write-Host "🧪 Running tests..." -ForegroundColor Cyan
try {
    cargo test --verbose
    Write-Status "All tests passed"
} catch {
    Write-Error "Some tests failed"
    exit 1
}

# Build workspace
Write-Host "🔨 Building workspace (debug)..." -ForegroundColor Cyan
try {
    cargo build --verbose
    Write-Status "Debug build successful"
} catch {
    Write-Error "Debug build failed"
    exit 1
}

Write-Host "🔨 Building workspace (release)..." -ForegroundColor Cyan
try {
    cargo build --release --verbose
    Write-Status "Release build successful"
} catch {
    Write-Error "Release build failed"
    exit 1
}

# Build example project
Write-Host "🎮 Building example project..." -ForegroundColor Cyan
try {
    cargo build --release --manifest-path examples/dodge-the-creeps-2d/rust/Cargo.toml
    Write-Status "Example project build successful"
} catch {
    Write-Error "Example project build failed"
    exit 1
}

# Check if built libraries exist
Write-Host "📋 Checking built artifacts..." -ForegroundColor Cyan

if (Test-Path "target/release/rust.dll") {
    Write-Status "Rust library built successfully: target/release/rust.dll"
} elseif (Test-Path "target/release/librust.dll") {
    Write-Status "Rust library built successfully: target/release/librust.dll"
} else {
    Write-Warning "Expected library file not found. Checking target/release/ contents:"
    Get-ChildItem "target/release/" -Filter "*.dll" | Format-Table Name, Length, LastWriteTime
}

Write-Host ""
Write-Status "✨ Local build completed successfully!"
Write-Host ""
Write-Host "📝 Next steps:" -ForegroundColor Cyan
Write-Host "   1. Test your changes locally"
Write-Host "   2. Commit and push to trigger CI"
Write-Host "   3. Check GitHub Actions for multi-platform builds"
Write-Host ""
Write-Host "🎮 To test with Godot:" -ForegroundColor Cyan
Write-Host "   1. Install Godot 4.4"
Write-Host "   2. Open examples/dodge-the-creeps-2d/godot/ in Godot"
Write-Host "   3. Run the project to test the integration" 