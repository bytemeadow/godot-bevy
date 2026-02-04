import inspect
import subprocess
from pathlib import Path


def indent_log(message):
    # Subtract 1 to ignore the current 'indent_log' frame
    # You might subtract more depending on your entry point
    depth = len(inspect.stack()) - 1 - 2

    # Create the indentation string (e.g., 2 spaces per level)
    indent = "    " * depth
    print(f"{indent}{message}")


def run_cargo_fmt(file_path: Path, project_root: Path) -> None:
    """Run cargo fmt on a specific file to format generated Rust code"""
    try:
        # Run cargo fmt on the specific file
        result = subprocess.run(
            ["cargo", "fmt", "--", str(file_path)],
            cwd=project_root,
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode == 0:
            indent_log(f"  ✓ Formatted {file_path.name}")
        else:
            indent_log(f"  ⚠ cargo fmt warning for {file_path.name}: {result.stderr}")

    except FileNotFoundError:
        indent_log(
            f"  ⚠ cargo fmt not found - skipping formatting for {file_path.name}"
        )
    except subprocess.TimeoutExpired:
        indent_log(f"  ⚠ cargo fmt timed out for {file_path.name}")
    except Exception as e:
        indent_log(f"  ⚠ Could not format {file_path.name}: {e}")
