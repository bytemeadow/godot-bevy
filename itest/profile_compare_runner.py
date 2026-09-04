#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

from profile_compare import (
    ComparisonError,
    ProfileInput,
    create_comparison,
    load_profile,
    print_table,
    write_comparison,
)
from profile_orchestrator import (
    ProfileFailure,
    display_path,
    make_run_id,
    re_safe_name,
    resolve_executable,
)
from profile_schema import (
    COMPARISON_SCHEMA_NAME,
    DISCLOSURE,
    SCHEMA_NAME,
    SchemaValidationError,
)

LOCAL_PACKAGES = (
    "godot-bevy",
    "godot-bevy-macros",
    "godot-bevy-test",
    "godot-bevy-test-macros",
    "godot-bevy-itest",
)


class RunnerError(RuntimeError):
    pass


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Capture and compare interleaved Tracy profiles."
    )
    parser.add_argument("base_ref", nargs="?", default="main")
    parser.add_argument("--self", action="store_true", dest="self_comparison")
    parser.add_argument("--bench", required=True, help="exact benchmark name")
    parser.add_argument("--output", type=Path, help="comparison artifact directory")
    return parser


def rounds_from_environment() -> int:
    value = os.environ.get("PROFILE_ROUNDS", "3")
    if not value.isdigit() or int(value) < 3:
        raise RunnerError("PROFILE_ROUNDS must be an integer of at least 3")
    return int(value)


def run_checked(command: list[str], cwd: Path, label: str) -> None:
    result = subprocess.run(command, cwd=cwd, check=False)
    if result.returncode != 0:
        raise RunnerError(f"{label} failed with exit {result.returncode}")


def clean_local_crates(cargo: str, repository: Path, manifest: Path, target: Path) -> None:
    command = [
        cargo,
        "clean",
        "--profile",
        "profiling",
        "--target-dir",
        str(target),
        "--manifest-path",
        str(manifest),
    ]
    for package in LOCAL_PACKAGES:
        command.extend(["-p", package])
    run_checked(command, repository, "profiling clean")


def build_side(cargo: str, source: Path, target: Path, label: str) -> None:
    manifest = source / "itest" / "rust" / "Cargo.toml"
    clean_local_crates(cargo, source, manifest, target)
    run_checked(
        [
            cargo,
            "build",
            "--profile",
            "profiling",
            "--manifest-path",
            str(manifest),
            "--target-dir",
            str(target),
            "--features",
            "profile-tracy",
        ],
        source,
        f"{label} profiling build",
    )


def library_name() -> str:
    if sys.platform == "darwin":
        return "libgodot_bevy_itest.dylib"
    if sys.platform == "win32":
        return "godot_bevy_itest.dll"
    return "libgodot_bevy_itest.so"


def stage_library(target: Path, destination: Path) -> None:
    source = target / "profiling" / library_name()
    if not source.is_file():
        raise RunnerError(f"profiling build did not produce {source}")
    destination.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination / source.name)


def add_worktree(repository: Path, path: Path, ref: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    run_checked(
        ["git", "worktree", "add", "--detach", str(path), ref],
        repository,
        f"create baseline worktree for {ref}",
    )


def remove_worktree(repository: Path, path: Path) -> None:
    if not path.exists():
        return
    subprocess.run(
        ["git", "worktree", "remove", str(path), "--force"],
        cwd=repository,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )


def capture_profile(
    orchestrator: Path,
    source: Path,
    target: Path,
    library: Path,
    output: Path,
    benchmark: str,
    label: str,
) -> Path:
    print(f"Capturing {label} ...", flush=True)
    command = [
        sys.executable,
        str(orchestrator),
        "--bench",
        benchmark,
        "--output",
        str(output),
        "--repository",
        str(source),
        "--target-dir",
        str(target),
        "--library-dir",
        str(library),
        "--skip-build",
    ]
    result = subprocess.run(
        command,
        cwd=source,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if result.returncode != 0:
        if result.stdout:
            print(result.stdout, file=sys.stderr, end="")
        raise RunnerError(f"{label} capture failed with exit {result.returncode}")
    spans = output / "spans.json"
    if not spans.is_file():
        raise RunnerError(f"{label} capture did not produce {spans}")
    return spans


def prepare_output(path: Path) -> None:
    if path.exists() and (not path.is_dir() or any(path.iterdir())):
        raise RunnerError(f"output directory already exists and is nonempty: {path}")
    path.mkdir(parents=True, exist_ok=True)


def run(args: argparse.Namespace) -> int:
    repository = Path(__file__).resolve().parents[1]
    if args.self_comparison and args.base_ref != "main":
        raise RunnerError("--self cannot be combined with a base ref")
    benchmark = args.bench.strip()
    if not benchmark:
        raise RunnerError("--bench must not be empty")
    rounds = rounds_from_environment()
    run_id, _ = make_run_id(repository)
    selector = re_safe_name(benchmark)
    if not selector:
        raise RunnerError("benchmark name has no safe path characters")
    output = (
        args.output.resolve()
        if args.output is not None
        else repository / "target" / "profiles" / "compare" / selector / run_id
    )
    prepare_output(output)

    cargo = resolve_executable(os.environ.get("CARGO", "cargo"), "Cargo")
    target = repository / "target"
    worktree = target / "profile-worktrees" / run_id
    staging = target / "profiles" / ".comparison-build" / run_id
    baseline_source = repository
    worktree_added = False
    try:
        if not args.self_comparison:
            add_worktree(repository, worktree, args.base_ref)
            baseline_source = worktree
            worktree_added = True

        print(f"Building baseline ({'self' if args.self_comparison else args.base_ref}) ...")
        build_side(cargo, baseline_source, target, "baseline")
        baseline_library = staging / "baseline"
        stage_library(target, baseline_library)

        print("Building current ...")
        build_side(cargo, repository, target, "current")
        current_library = staging / "current"
        stage_library(target, current_library)

        orchestrator = repository / "itest" / "profile_orchestrator.py"
        profile_schema = repository / "itest" / "schema" / SCHEMA_NAME
        comparison_schema = (
            repository / "itest" / "schema" / COMPARISON_SCHEMA_NAME
        )
        inputs: list[ProfileInput] = []
        for round_number in range(1, rounds + 1):
            for side, source, library in (
                ("baseline", baseline_source, baseline_library),
                ("current", repository, current_library),
            ):
                profile_path = capture_profile(
                    orchestrator,
                    source,
                    target,
                    library,
                    output / f"{side}-run{round_number}",
                    benchmark,
                    f"round {round_number}/{rounds} {side}",
                )
                inputs.append(
                    load_profile(profile_path, profile_schema, side, round_number)
                )

        document = create_comparison(inputs, "interleaved", run_id)
        comparison_path = output / "comparison.json"
        write_comparison(document, comparison_path, comparison_schema)
        print_table(document)
        print(f"Comparison complete: {display_path(comparison_path, repository)}")
    finally:
        if staging.exists():
            shutil.rmtree(staging)
        if worktree_added:
            remove_worktree(repository, worktree)
    return 0


def main() -> int:
    args = create_parser().parse_args()
    print(DISCLOSURE, flush=True)
    try:
        return run(args)
    except (
        RunnerError,
        ComparisonError,
        ProfileFailure,
        SchemaValidationError,
        OSError,
    ) as error:
        print(f"Comparison failed: {error}", file=sys.stderr)
        return 2
    except KeyboardInterrupt:
        print("Comparison interrupted", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
