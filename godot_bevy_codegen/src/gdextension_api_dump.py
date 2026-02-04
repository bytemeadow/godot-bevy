import json
import subprocess
from pathlib import Path

import dacite

from godot_bevy_codegen.src.gdextension_api import ExtensionApi
from godot_bevy_codegen.src.util import indent_log


def run_godot_dump_api(destination_file: Path, godot_version: str) -> None:
    """Run godot --dump-extension-api-with-docs to generate extension_api.json"""
    indent_log("🚀 Generating extension_api.json from Godot...")

    try:
        if destination_file.exists():
            indent_log(f"✅ '{destination_file}' already exists, skipping generation")
            return

        switch_to_godot_version(godot_version)

        # Try different common Godot executable names
        godot_commands = [
            "godot",
            "godot4",
            "/usr/local/bin/godot",
            Path.home() / ".local/share/gdenv/bin/godot",
        ]

        destination_file.parent.mkdir(parents=True, exist_ok=True)

        godot_output_file = Path("extension_api.json")

        for cmd in godot_commands:
            try:
                result = subprocess.run(
                    [
                        cmd,
                        "--headless",
                        "--dump-extension-api-with-docs",
                    ],
                    capture_output=True,
                    text=True,
                    timeout=30,
                )

                if result.returncode == 0 and godot_output_file.exists():
                    # Relocate Godot's output file to the destination directory
                    godot_output_file.rename(destination_file)
                    indent_log(
                        f"✅ Successfully generated '{destination_file}' using '{cmd}'"
                    )
                    return

            except (subprocess.TimeoutExpired, FileNotFoundError):
                continue

        # If all commands failed, give helpful error
        raise RuntimeError(
            "Could not run Godot to generate extension_api.json.\n"
            "Please ensure Godot 4 is installed and available in PATH."
        )

    except Exception as e:
        raise RuntimeError(f"Error generating {destination_file}") from e


def switch_to_godot_version(godot_version: str) -> None:
    try:
        subprocess.run(
            [
                "gdenv",
                "install",
                godot_version,
            ]
        )
        subprocess.run(
            [
                "gdenv",
                "use",
                godot_version,
            ]
        )
    except Exception as e:
        raise RuntimeError(f"Error switching to Godot version {godot_version}") from e


def load_extension_api(
    api_file: Path,
) -> ExtensionApi:
    """Load and parse the extension API to extract node types"""
    indent_log("📖 Parsing extension API...")

    if not api_file.exists():
        raise FileNotFoundError(f"extension_api.json not found at {api_file}")

    with open(api_file) as f:
        json_object = json.load(f)

    return dacite.from_dict(ExtensionApi, json_object)
