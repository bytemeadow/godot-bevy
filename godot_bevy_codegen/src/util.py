import inspect
import subprocess
from pathlib import Path
from typing import List


def make_indent_log():
    # Capture the first call depth so wrapper frames do not affect indentation.
    first_stack_depth = None

    def log(message):
        nonlocal first_stack_depth
        if first_stack_depth is None:
            first_stack_depth = len(inspect.stack())
        depth = len(inspect.stack()) - first_stack_depth

        indent = "    " * depth
        print(f"{indent}{message}")

    return log


indent_log = make_indent_log()


def run_cargo_fmt(file_paths: List[Path], project_root: Path) -> None:
    try:
        result = subprocess.run(
            ["cargo", "fmt", "--", *file_paths],
            cwd=project_root,
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode == 0:
            indent_log(f"  ✓ Formatted rust files")
        else:
            indent_log(f"  ⚠ cargo fmt warning")

    except FileNotFoundError as e:
        indent_log(f"  ⚠ cargo fmt not found - skipping formatting")
        raise e
    except subprocess.TimeoutExpired as e:
        indent_log(f"  ⚠ cargo fmt timed out")
        raise e
    except Exception as e:
        indent_log(f"  ⚠ Could not format rust files")
        raise e
