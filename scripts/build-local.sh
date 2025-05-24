#!/bin/bash

# Local build script to test CI pipeline locally
# This script mimics the CI workflow for local testing

set -e

echo "🚀 Starting local build process..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo_status() {
    echo -e "${GREEN}✓${NC} $1"
}

echo_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

echo_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    echo_error "Error: Please run this script from the project root directory"
    exit 1
fi

echo "📦 Building Rust workspace..."

# Format check
echo "🔍 Checking code formatting..."
if cargo fmt --all -- --check; then
    echo_status "Code formatting is correct"
else
    echo_error "Code formatting issues found. Run 'cargo fmt' to fix."
    exit 1
fi

# Clippy check
echo "🔍 Running clippy..."
if cargo clippy --all-targets --all-features -- -D warnings; then
    echo_status "Clippy checks passed"
else
    echo_error "Clippy found issues"
    exit 1
fi

# Run tests
echo "🧪 Running tests..."
if cargo test --verbose; then
    echo_status "All tests passed"
else
    echo_error "Some tests failed"
    exit 1
fi

# Build workspace
echo "🔨 Building workspace (debug)..."
if cargo build --verbose; then
    echo_status "Debug build successful"
else
    echo_error "Debug build failed"
    exit 1
fi

echo "🔨 Building workspace (release)..."
if cargo build --release --verbose; then
    echo_status "Release build successful"
else
    echo_error "Release build failed"
    exit 1
fi

# Build example project
echo "🎮 Building example project..."
if cargo build --release --manifest-path examples/dodge-the-creeps-2d/rust/Cargo.toml; then
    echo_status "Example project build successful"
else
    echo_error "Example project build failed"
    exit 1
fi

# Check if built libraries exist
echo "📋 Checking built artifacts..."
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    LIB_EXT=".so"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    LIB_EXT=".dylib"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    LIB_EXT=".dll"
else
    echo_warning "Unknown OS type: $OSTYPE"
    LIB_EXT=".so"  # Default fallback
fi

if [[ -f "target/release/librust${LIB_EXT}" ]]; then
    echo_status "Rust library built successfully: target/release/librust${LIB_EXT}"
else
    # Check for Windows naming convention
    if [[ -f "target/release/rust.dll" ]]; then
        echo_status "Rust library built successfully: target/release/rust.dll"
    else
        echo_warning "Expected library file not found. Checking target/release/ contents:"
        ls -la target/release/ | grep -E '\.(so|dylib|dll)$' || echo "No library files found"
    fi
fi

echo ""
echo_status "✨ Local build completed successfully!"
echo ""
echo "📝 Next steps:"
echo "   1. Test your changes locally"
echo "   2. Commit and push to trigger CI"
echo "   3. Check GitHub Actions for multi-platform builds"
echo ""
echo "🎮 To test with Godot:"
echo "   1. Install Godot 4.4"
echo "   2. Open examples/dodge-the-creeps-2d/godot/ in Godot"
echo "   3. Run the project to test the integration" 