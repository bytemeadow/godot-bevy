#!/usr/bin/env python3
"""
Fully automatic Godot type generation for godot-bevy.

This script:
1. Runs `godot --dump-extension-api-with-docs` to generate extension_api.json
2. Parses all Node-derived types from the API
3. Generates comprehensive node marker components
4. Generates complete type checking functions
5. Updates the scene tree plugin to use generated code

Usage: uv run python -m godot_bevy_codegen
"""
import textwrap

from godot_bevy_codegen.src.file_paths import FilePaths
from godot_bevy_codegen.src.gdextension_api_dump import (
    run_godot_dump_api,
    load_extension_api,
)
from godot_bevy_codegen.src.gen_gdscript_watcher import (
    generate_gdscript_watcher,
    use_watcher_version,
)
from godot_bevy_codegen.src.gen_signal_names import generate_signal_names
from godot_bevy_codegen.src.gen_type_checking import (
    generate_type_checking_code,
)
from godot_bevy_codegen.src.gen_node_markers import generate_node_markers
from godot_bevy_codegen.src.util import (
    indent_log,
)


def generate_for_version(api_version: str) -> None:
    indent_log("Step 1: Generate extension API")
    run_godot_dump_api(
        FilePaths.extension_api_file(api_version),
        api_version,
    )

    indent_log("Step 2: Parse API and extract types")
    api = load_extension_api(FilePaths.extension_api_file(api_version))

    indent_log("Step 3: Generate node markers")
    generate_node_markers(
        FilePaths.node_markers_file(api_version),
        FilePaths.project_root,
        api,
    )

    indent_log("Step 4: Generate type checking code")
    generate_type_checking_code(
        FilePaths.type_checking_file(api_version),
        FilePaths.project_root,
        api,
    )

    indent_log("Step 5: Generate optimized GDScript watcher")
    generate_gdscript_watcher(
        FilePaths.gdscript_watcher_file(api_version),
        api,
    )

    indent_log("Step 6: Generate signal names")
    generate_signal_names(
        FilePaths.signal_names_file(api_version),
        FilePaths.project_root,
        api,
    )


def main() -> None:
    """Run the complete generation pipeline"""
    indent_log("🎯 Starting Godot type generation pipeline...")

    # The Godot versions used here are sourced from Godot-Rust's handling of gdextension API differences:
    # https://github.com/godot-rust/gdext/blob/3f1d543580c1817f1b7fab57a400e82b50085581/godot-bindings/src/import.rs
    # Check the main branch for latest versions: https://github.com/godot-rust/gdext/blob/master/godot-bindings/src/import.rs
    api_versions = ["4.2", "4.2.1", "4.2.2", "4.3", "4.4", "4.5"]

    try:
        for api_version in api_versions:
            indent_log(f"⚙️  Processing API version {api_version}...")
            generate_for_version(api_version)

        # Use the most recent version as the active OptimizedSceneTreeWatcher
        use_watcher_version(api_versions[-1])

        indent_log("")
        indent_log("🎉 Generation complete!")
        indent_log("")
        indent_log("    Files generated:")
        for path in FilePaths.all_generated_files(api_versions):
            indent_log(f"       • {path.relative_to(FilePaths.project_root)}")
        indent_log(
            textwrap.dedent(
                f"""
            Next steps:
              • Run 'cargo check' to verify the build
              • Update the following files with the latest versions:
                • godot-bevy/src/plugins/scene_tree/node_type_checking.rs
                • godot-bevy/src/interop/node_markers.rs
                • godot-bevy/src/interop/signal_names.rs
              • Commit the generated files
            """
            )
        )

    except Exception as e:
        raise RuntimeError("Generation failed") from e


if __name__ == "__main__":
    main()
