#!/usr/bin/env python3
import os
import re
import signal
import subprocess
import sys


def main():
    timeout = int(os.environ.get("GODOT_TIMEOUT_SECONDS", "300"))
    process = subprocess.Popen(
        sys.argv[1:],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=True,
        text=True,
        errors="replace",
    )
    try:
        output, _ = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        if os.name == "posix":
            os.killpg(process.pid, signal.SIGKILL)
        else:
            process.kill()
        output, _ = process.communicate()
        print(output, end="", flush=True)
        print(f"Godot timed out after {timeout}s", file=sys.stderr)
        return 2
    print(output, end="", flush=True)
    if re.search(
        r"SCRIPT ERROR:|Parse Error:|Failed to load (?:script|resource)|"
        r"Error loading (?:extension|GDExtension)|Can't open dynamic library|"
        r"No GDExtension library found",
        output,
    ):
        print("Godot reported a script or load error", file=sys.stderr)
        return 2
    return process.returncode if process.returncode >= 0 else 128 - process.returncode


if __name__ == "__main__":
    sys.exit(main())
