# godot-bevy Scripts

This directory contains utility scripts for maintaining the godot-bevy codebase.

## generate_godot_types.py

This Python script provides **fully automatic** generation of comprehensive node marker components and type checking code from Godot's extension API.

### Purpose

The script ensures that godot-bevy has marker components for ALL Godot node types, not just a manually maintained subset. This is important for:

1. **Completeness**: Users can query for any Godot node type in their Bevy systems
2. **Maintainability**: When Godot adds new node types, we can regenerate everything automatically
3. **Accuracy**: All code is generated directly from Godot's official API definition
4. **Zero Manual Work**: The entire pipeline is automated - no manual file editing required

### Usage

```bash
# From the project root - this does EVERYTHING automatically
python scripts/generate_godot_types.py
```

### What it does (fully automatic)

1. **Runs `godot --dump-extension-api`** to generate the latest API definition
2. **Parses all Node-derived classes** from the API (247 total types)
3. **Filters out problematic classes** (editor-only, module-specific, etc.)
4. **Generates `node_markers.rs`** with marker components for all 247 types
5. **Generates comprehensive type checking code** in a separate file
6. **Automatically updates the scene tree plugin** to use the generated code
7. **Handles class name mappings** (e.g., `HTTPRequest` → `HttpRequest`)

### Files Generated/Updated

- `godot-bevy/src/interop/node_markers.rs` - All 247 marker components
- `godot-bevy/src/plugins/scene_tree/node_type_checking_generated.rs` - Complete type checking
- `godot-bevy/src/plugins/scene_tree/plugin.rs` - Updated to use generated code
- `godot-bevy/src/plugins/scene_tree/mod.rs` - Module declarations
- `extension_api.json` - Fresh API dump from Godot

### Prerequisites

- Python 3.6+
- Godot 4 installed and available in PATH (as `godot`, `godot4`, or `/usr/local/bin/godot`)

### When to run

- **After updating to a new Godot version** - get all new node types automatically
- **When contributing to the project** - ensure comprehensive type coverage
- **To verify completeness** - see exactly which types are supported

### Benefits over manual approach

- **🚀 Fully automated** - no manual steps or file editing
- **🎯 Comprehensive** - covers ALL 247 node types (vs ~40 manually maintained)
- **🔧 Maintainable** - single command to update for new Godot versions
- **✅ Reliable** - generated from official Godot API, handles name mappings
- **📦 Complete** - generates markers AND type checking AND plugin integration
