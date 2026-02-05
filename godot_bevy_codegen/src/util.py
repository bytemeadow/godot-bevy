import inspect
import subprocess
from pathlib import Path


def make_indent_log():
    # The stack can have several frames before the 'indent_log' function is called,
    # this will normalize the indentation depth
    first_stack_depth = None

    def log(message):
        nonlocal first_stack_depth
        if first_stack_depth is None:
            first_stack_depth = len(inspect.stack())
        depth = len(inspect.stack()) - first_stack_depth

        # Create the indentation string (e.g., 2 spaces per level)
        indent = "    " * depth
        print(f"{indent}{message}")

    return log


indent_log = make_indent_log()


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
