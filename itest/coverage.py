#!/usr/bin/env python3
from __future__ import annotations

import argparse
import errno
import gzip
import json
import os
import platform
import re
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import uuid
from contextlib import contextmanager
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterator

from coverage_model import (
    COVERAGE_SOURCES,
    CoverageIndex,
    CoverageModelError,
    ScopeConfig,
    SourceEntry,
    classify_diff,
    diff_exit,
    empty_coverage_counts,
    evaluate_witness,
    inline_test_modules,
    inventory_sources,
    load_scope_config,
    load_witnesses,
    parse_cargo_json,
    parse_flush_sentinel,
    parse_itest_report,
    parse_llvm_cov_export,
    parse_unified_diff,
    relative_path,
    select_cargo_objects,
    sha256_bytes,
    sha256_file,
    source_identity,
    state_counts,
    sum_coverage_counts,
    validate_coverage_document,
    validate_mapping_identity,
)


REPOSITORY = Path(__file__).resolve().parents[1]
COVERAGE_ROOT = REPOSITORY / "target" / "coverage"
BUILD_ROOT = COVERAGE_ROOT / "build"
RUNS_ROOT = COVERAGE_ROOT / "runs"
LOCK_PATH = COVERAGE_ROOT / ".lock"
LATEST_PATH = COVERAGE_ROOT / "latest-run.txt"
SCOPE_PATH = REPOSITORY / "itest" / "coverage" / "scope-v1.toml"
WITNESSES_PATH = REPOSITORY / "itest" / "coverage" / "witnesses-v1.toml"
SCHEMA_PATH = REPOSITORY / "itest" / "schema" / "coverage-v1.schema.json"
ITEST_SCHEMA_PATH = REPOSITORY / "godot-bevy-test" / "schema" / "itest-report-v1.schema.json"
GODOT_PROJECT = REPOSITORY / "itest" / "godot"
GDEXTENSION_PATH = GODOT_PROJECT / "itest.gdextension"
EXTENSION_LIST_PATH = GODOT_PROJECT / ".godot" / "extension_list.cfg"
ITEST_FEATURES = ("autosync-tests", "coverage-flush", "test-frame-signal")
CONTROL_PATHS = (
    Path("Cargo.toml"),
    Path("devenv.nix"),
    Path("godot-bevy-test/schema/coverage-v1.schema.json"),
    Path("itest/coverage.py"),
    Path("itest/coverage_model.py"),
    Path("itest/coverage/scope-v1.toml"),
    Path("itest/coverage/witnesses-v1.toml"),
    Path("itest/rust/Cargo.toml"),
    Path("itest/rust/src/coverage_flush.rs"),
    Path("itest/rust/src/lib.rs"),
)
PHASE_DEFINITIONS = (
    ("unit-build", True, "test-build"),
    ("unit-runtime", True, "unit-runtime"),
    ("itest-build", True, "test-build"),
    ("import", False, "excluded"),
    ("itest-runtime", True, "itest-runtime"),
)


class CoverageInfrastructureError(RuntimeError):
    def __init__(self, message: str, phase: str | None = None):
        super().__init__(message)
        self.phase = phase


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def new_run_id() -> str:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    return f"{stamp}-{os.getpid()}-{uuid.uuid4().hex[:8]}"


def _cleanup_hint(message: str) -> str:
    return f"{message}; run `devenv shell -- coverage clean`"


def _storage_failure(output: str) -> bool:
    return "No space left on device" in output or "ENOSPC" in output


def infrastructure_error(
    error: BaseException, phase: str | None = None
) -> CoverageInfrastructureError:
    if isinstance(error, CoverageInfrastructureError):
        return error
    if isinstance(error, OSError) and error.errno == errno.ENOSPC:
        return CoverageInfrastructureError(_cleanup_hint("coverage storage is full"), phase)
    return CoverageInfrastructureError(str(error), phase)


def _relative(path: Path) -> str:
    return relative_path(path, REPOSITORY)


def _write_bytes_atomic(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        descriptor, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
        temporary = Path(name)
        with os.fdopen(descriptor, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
    except OSError as error:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
        raise infrastructure_error(error, "write") from error


def write_json(path: Path, document: Any) -> None:
    payload = json.dumps(
        document, indent=2, sort_keys=False, allow_nan=False
    ).encode()
    _write_bytes_atomic(path, payload + b"\n")


def directory_size(path: Path) -> int:
    total = 0
    if not path.exists():
        return 0
    for entry in path.rglob("*"):
        try:
            if entry.is_file() and not entry.is_symlink():
                total += entry.stat().st_size
        except OSError as error:
            raise infrastructure_error(error, "operations") from error
    return total


def free_disk(path: Path) -> int:
    anchor = path if path.exists() else path.parent
    return shutil.disk_usage(anchor).free


def _remove_path(path: Path) -> None:
    if path.is_symlink() or path.is_file():
        path.unlink(missing_ok=True)
    elif path.is_dir():
        shutil.rmtree(path)


def _pid_is_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def _validate_coverage_paths() -> None:
    expected = REPOSITORY.resolve() / "target" / "coverage"
    actual = COVERAGE_ROOT.parent.resolve() / COVERAGE_ROOT.name
    if actual != expected or COVERAGE_ROOT.is_symlink():
        raise CoverageInfrastructureError("unsafe coverage root", "operations")
    for path, name in ((BUILD_ROOT, "build"), (RUNS_ROOT, "runs")):
        if path != COVERAGE_ROOT / name or path.is_symlink():
            raise CoverageInfrastructureError(f"unsafe coverage path: {path}", "operations")


class CoverageLock:
    def __enter__(self) -> CoverageLock:
        _validate_coverage_paths()
        COVERAGE_ROOT.mkdir(parents=True, exist_ok=True)
        try:
            LOCK_PATH.mkdir()
        except FileExistsError as error:
            raise CoverageInfrastructureError(
                _cleanup_hint("another coverage run owns target/coverage/.lock"),
                "lock",
            ) from error
        try:
            write_json(
                LOCK_PATH / "owner.json",
                {"schema_version": 1, "pid": os.getpid(), "started_at": utc_now()},
            )
        except BaseException:
            _remove_path(LOCK_PATH)
            raise
        return self

    def __exit__(self, exc_type: Any, exc: Any, traceback: Any) -> None:
        try:
            (LOCK_PATH / "owner.json").unlink(missing_ok=True)
            LOCK_PATH.rmdir()
        except OSError as error:
            if exc_type is None:
                raise CoverageInfrastructureError(
                    _cleanup_hint(f"could not release the coverage lock: {error}"),
                    "lock",
                ) from error


def clean_coverage() -> int:
    try:
        _validate_coverage_paths()
    except CoverageInfrastructureError as error:
        print(f"ERROR coverage clean: {error}", file=sys.stderr)
        return 2
    if LOCK_PATH.exists():
        try:
            owner = json.loads((LOCK_PATH / "owner.json").read_text(encoding="utf-8"))
            pid = owner.get("pid") if isinstance(owner, dict) else None
        except (OSError, json.JSONDecodeError):
            pid = None
        if not isinstance(pid, int) or isinstance(pid, bool) or pid <= 0:
            print("ERROR coverage clean: coverage lock owner is invalid", file=sys.stderr)
            return 2
        if _pid_is_alive(pid):
            print(f"ERROR coverage clean: coverage run {pid} is active", file=sys.stderr)
            return 2
        _remove_path(LOCK_PATH)
    for path in (BUILD_ROOT, RUNS_ROOT, LATEST_PATH):
        try:
            _remove_path(path)
        except OSError as error:
            print(f"ERROR coverage clean: {error}", file=sys.stderr)
            return 2
    try:
        COVERAGE_ROOT.rmdir()
    except OSError:
        pass
    print("PASS coverage clean: removed target/coverage build and runs")
    return 0


def _run_text(arguments: list[str], env: dict[str, str] | None = None) -> str:
    process = subprocess.Popen(
        arguments,
        cwd=REPOSITORY,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=True,
    )
    try:
        stdout, stderr = process.communicate(timeout=60)
    except subprocess.TimeoutExpired as error:
        _terminate_process(process)
        raise CoverageInfrastructureError(
            f"{' '.join(arguments)} timed out after 60s", "tools"
        ) from error
    except BaseException:
        _terminate_process(process)
        raise
    if process.returncode != 0:
        detail = stderr.strip() or stdout.strip()
        if _storage_failure(detail):
            raise CoverageInfrastructureError(
                _cleanup_hint("coverage subprocess ran out of storage"), "tools"
            )
        raise CoverageInfrastructureError(
            f"{' '.join(arguments)} failed with {process.returncode}: {detail}",
            "tools",
        )
    return stdout


def _git_bytes(*arguments: str) -> bytes:
    result = subprocess.run(
        ["git", *arguments],
        cwd=REPOSITORY,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.decode(errors="replace").strip()
        raise CoverageInfrastructureError(
            f"git {' '.join(arguments)} failed: {detail}", "identity"
        )
    return result.stdout


def _git_text(*arguments: str) -> str:
    return _git_bytes(*arguments).decode("utf-8")


def _worktree_identity(source_digest: str) -> tuple[str, bytes]:
    status = _git_bytes("status", "--porcelain=v1", "-z", "--untracked-files=all")
    diff = _git_bytes("diff", "--no-ext-diff", "--binary", "HEAD", "--")
    return sha256_bytes(status + b"\0" + diff + b"\0" + source_digest.encode()), status


def _configured_int(name: str, default: int, minimum: int = 1) -> int:
    raw = os.environ.get(name, str(default))
    try:
        value = int(raw)
    except ValueError as error:
        raise CoverageInfrastructureError(f"{name} must be an integer", "config") from error
    if value < minimum:
        raise CoverageInfrastructureError(f"{name} must be at least {minimum}", "config")
    return value


def _parse_show_env(output: str, inherited: dict[str, str]) -> dict[str, str]:
    if not output or not output.endswith("\n"):
        raise CoverageInfrastructureError(
            "cargo llvm-cov show-env output is not newline-terminated", "tools"
        )
    environment = inherited.copy()
    exported: set[str] = set()
    for line_number, line in enumerate(output.splitlines(), 1):
        if not line or line != line.strip() or not line.startswith("export "):
            raise CoverageInfrastructureError(
                f"cargo llvm-cov show-env line {line_number} is not an export", "tools"
            )
        tokens = shlex.split(line)
        if len(tokens) != 2 or "=" not in tokens[1]:
            raise CoverageInfrastructureError(
                f"cargo llvm-cov show-env line {line_number} is invalid", "tools"
            )
        name, value = tokens[1].split("=", 1)
        if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name) is None:
            raise CoverageInfrastructureError("cargo llvm-cov exported an invalid name", "tools")
        if name in exported:
            raise CoverageInfrastructureError(
                f"cargo llvm-cov exported {name} more than once", "tools"
            )
        exported.add(name)
        environment[name] = value
    return environment


def _version_from(text: str, label: str) -> str:
    match = re.search(r"LLVM version:?\s+([0-9]+(?:\.[0-9]+)+)", text)
    if match is None:
        raise CoverageInfrastructureError(f"could not parse {label} LLVM version", "tools")
    return match.group(1)


def _coverage_flags(environment: dict[str, str]) -> str:
    names = (
        "__CARGO_LLVM_COV_RUSTC_WRAPPER_RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTFLAGS",
    )
    return " ".join(environment.get(name, "").replace("\x1f", " ") for name in names)


def _inherited_wrapper_preserved(
    inherited: dict[str, str], environment: dict[str, str]
) -> bool:
    wrapper = inherited.get("RUSTC_WRAPPER")
    if wrapper is None:
        return "RUSTC_WRAPPER" not in environment or environment.get(
            "__CARGO_LLVM_COV_RUSTC_WRAPPER"
        ) == "1"
    if environment.get("RUSTC_WRAPPER") == wrapper:
        return True
    return (
        environment.get("__CARGO_LLVM_COV_RUSTC_WRAPPER") == "1"
        and environment.get("__CARGO_LLVM_COV_RUSTC_WRAPPER_PRE_EXISTING") == wrapper
    )


def _validate_coverage_directories(
    environment: dict[str, str], repository: Path, build_root: Path
) -> None:
    expected = build_root.resolve()
    for name in ("CARGO_LLVM_COV_TARGET_DIR", "CARGO_LLVM_COV_BUILD_DIR"):
        value = environment.get(name)
        if value is None:
            raise CoverageInfrastructureError(
                f"cargo llvm-cov did not export {name}", "tools"
            )
        path = Path(value)
        if not path.is_absolute():
            path = repository / path
        if path.resolve() != expected:
            raise CoverageInfrastructureError(
                f"cargo llvm-cov exported an unsafe {name}: {value}", "tools"
            )


def resolve_tools() -> tuple[dict[str, str], dict[str, str]]:
    inherited = os.environ.copy()
    if inherited.get("CARGO_LLVM_COV_SHOW_ENV") or inherited.get(
        "__CARGO_LLVM_COV_RUSTC_WRAPPER"
    ):
        raise CoverageInfrastructureError(
            "coverage cannot run inside an existing cargo llvm-cov environment", "tools"
        )
    inherited_flags = " ".join(
        value.replace("\x1f", " ")
        for name, value in inherited.items()
        if "RUSTFLAGS" in name or "RUSTDOCFLAGS" in name
    )
    if any(
        option in inherited_flags
        for option in ("instrument-coverage", "coverage-options=", "link-dead-code")
    ):
        raise CoverageInfrastructureError(
            "inherited Rust flags contain a conflicting coverage policy", "tools"
        )
    inherited["CARGO_LLVM_COV_TARGET_DIR"] = str(BUILD_ROOT)
    inherited["CARGO_LLVM_COV_BUILD_DIR"] = str(BUILD_ROOT)
    inherited["CARGO_TARGET_DIR"] = str(BUILD_ROOT)
    show_env = _run_text(["cargo", "llvm-cov", "show-env", "--sh"], inherited)
    environment = _parse_show_env(show_env, inherited)
    _validate_coverage_directories(environment, REPOSITORY, BUILD_ROOT)
    if not _inherited_wrapper_preserved(inherited, environment):
        raise CoverageInfrastructureError(
            "cargo llvm-cov did not preserve the inherited RUSTC_WRAPPER", "tools"
        )
    flags = _coverage_flags(environment)
    if "instrument-coverage" not in flags or "cfg=coverage" not in flags:
        raise CoverageInfrastructureError(
            "cargo llvm-cov environment lacks instrument-coverage or cfg=coverage", "tools"
        )
    wrapper_flags = environment.get(
        "__CARGO_LLVM_COV_RUSTC_WRAPPER_RUSTFLAGS", ""
    ).split("\x1f")
    if wrapper_flags != ["-C", "instrument-coverage", "--cfg=coverage"]:
        raise CoverageInfrastructureError(
            f"cargo llvm-cov exported unexpected instrumentation flags: {wrapper_flags}",
            "tools",
        )
    if any(option in flags for option in ("coverage-options=", "link-dead-code")):
        raise CoverageInfrastructureError("forbidden coverage flags are inherited", "tools")
    if "%c" in environment.get("LLVM_PROFILE_FILE", ""):
        raise CoverageInfrastructureError("continuous coverage mode is forbidden", "tools")
    environment["CARGO_INCREMENTAL"] = "0"
    environment["CARGO_TARGET_DIR"] = str(BUILD_ROOT)
    environment["DEBUGINFOD_URLS"] = ""
    environment["SCCACHE_RECACHE"] = "1"

    rustc_verbose = _run_text(["rustc", "--version", "--verbose"], environment)
    host_match = re.search(r"^host:\s*(\S+)$", rustc_verbose, re.MULTILINE)
    if host_match is None:
        raise CoverageInfrastructureError("rustc did not report a host triple", "tools")
    host = host_match.group(1)
    if host not in {"aarch64-apple-darwin", "x86_64-unknown-linux-gnu"}:
        raise CoverageInfrastructureError(f"unsupported coverage target: {host}", "tools")
    sysroot = Path(_run_text(["rustc", "--print", "sysroot"], environment).strip()).resolve()
    suffix = ".exe" if os.name == "nt" else ""
    tool_dir = sysroot / "lib" / "rustlib" / host / "bin"
    # nix sysroots are symlink forests: the entries under lib/rustlib/<host>/bin
    # point into the llvm-tools component's own store path, so validate the
    # sysroot-relative location before resolving for execution.
    for name in ("llvm-cov", "llvm-profdata"):
        if not (tool_dir / f"{name}{suffix}").is_file():
            raise CoverageInfrastructureError(f"{name} is not from the rustc sysroot", "tools")
    llvm_cov = (tool_dir / f"llvm-cov{suffix}").resolve()
    llvm_profdata = (tool_dir / f"llvm-profdata{suffix}").resolve()
    cargo_version = _run_text(["cargo", "llvm-cov", "--version"], environment).strip()
    if cargo_version != "cargo-llvm-cov 0.9.0":
        raise CoverageInfrastructureError(
            f"expected cargo-llvm-cov 0.9.0, got {cargo_version!r}", "tools"
        )
    cov_version_text = _run_text([str(llvm_cov), "--version"], environment)
    profdata_version_text = _run_text([str(llvm_profdata), "--version"], environment)
    rustc_llvm = _version_from(rustc_verbose, "rustc")
    cov_version = _version_from(cov_version_text, "llvm-cov")
    profdata_version = _version_from(profdata_version_text, "llvm-profdata")
    if len({rustc_llvm, cov_version, profdata_version}) != 1:
        raise CoverageInfrastructureError(
            "LLVM version mismatch: "
            f"rustc={rustc_llvm} cov={cov_version} profdata={profdata_version}",
            "tools",
        )
    return environment, {
        "host": host,
        "sysroot": str(sysroot),
        "rustc": rustc_verbose.splitlines()[0],
        "rustc_llvm": rustc_llvm,
        "llvm_cov": str(llvm_cov),
        "llvm_cov_version": cov_version,
        "llvm_profdata": str(llvm_profdata),
        "llvm_profdata_version": profdata_version,
        "cargo_llvm_cov": "0.9.0",
    }


def _terminate_process(process: subprocess.Popen[Any]) -> None:
    if process.poll() is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()


def _process(
    arguments: list[str],
    environment: dict[str, str],
    stdout_path: Path,
    stderr_path: Path,
    timeout_seconds: int,
    cwd: Path = REPOSITORY,
) -> dict[str, Any]:
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        process = subprocess.Popen(
            arguments,
            cwd=cwd,
            env=environment,
            stdout=stdout,
            stderr=stderr,
            start_new_session=True,
        )
        try:
            exit_code = process.wait(timeout=timeout_seconds)
        except subprocess.TimeoutExpired as error:
            _terminate_process(process)
            raise CoverageInfrastructureError(
                f"process timed out after {timeout_seconds}s: {' '.join(arguments)}",
                "process",
            ) from error
        except BaseException:
            _terminate_process(process)
            raise
    if exit_code != 0:
        output = ""
        for path in (stderr_path, stdout_path):
            try:
                output += path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                pass
        if _storage_failure(output):
            raise CoverageInfrastructureError(
                _cleanup_hint("coverage subprocess ran out of storage"), "process"
            )
    return {
        "pid": process.pid,
        "exit_code": exit_code,
        "elapsed_seconds": time.monotonic() - started,
        "output_bytes": stdout_path.stat().st_size + stderr_path.stat().st_size,
    }


def discover_profraw(directory: Path, expected_pid: int | None = None) -> list[Path]:
    if not directory.is_dir():
        raise CoverageInfrastructureError(f"profraw directory is missing: {directory}", "profiles")
    profiles = sorted(directory.rglob("*.profraw"))
    if not profiles:
        raise CoverageInfrastructureError(
            f"no profraw files in fresh directory {directory}", "profiles"
        )
    for profile in profiles:
        if not profile.is_file() or profile.stat().st_size == 0:
            raise CoverageInfrastructureError(f"profraw is empty: {profile}", "profiles")
    # tests may re-exec themselves (env-var probes); children inherit
    # LLVM_PROFILE_FILE and write real coverage into the same fresh directory,
    # so the spawned pid must be present but need not be alone.
    if expected_pid is not None and not any(
        profile.name.startswith(f"{expected_pid}-") for profile in profiles
    ):
        raise CoverageInfrastructureError(
            f"no profraw belongs to process {expected_pid}", "profiles"
        )
    return profiles


def _fresh_directory(path: Path) -> Path:
    try:
        path.mkdir(parents=True, exist_ok=False)
    except FileExistsError as error:
        raise CoverageInfrastructureError(
            f"phase directory is not fresh: {path}", "profiles"
        ) from error
    return path


def _phase_environment(base: dict[str, str], raw_directory: Path) -> dict[str, str]:
    environment = base.copy()
    environment["LLVM_PROFILE_FILE"] = str(raw_directory.resolve() / "%p-%m.profraw")
    return environment


def _artifact(path: Path, kind: str, metadata: dict[str, Any] | None = None) -> dict[str, Any]:
    return {
        "kind": kind,
        "path": _relative(path),
        "sha256": sha256_file(path),
        "size_bytes": path.stat().st_size,
        "retained": True,
        "metadata": {
            "instrumented": True,
            "language_scope": ["rust"],
            "godot_host_instrumented": False,
            "gdscript_instrumented": False,
            "benchmark_compatible": False,
            "timing_valid": False,
            "rate_gates": [],
            **(metadata or {}),
        },
    }


class CoverageRun:
    def __init__(self, mode: str, base: str | None, keep_raw: bool):
        self.mode = mode
        self.base = base
        self.keep_raw = keep_raw
        self.run_id = new_run_id()
        self.run_dir = RUNS_ROOT / self.run_id
        self.logs_dir = self.run_dir / "logs"
        self.raw_dir = self.run_dir / "raw"
        self.profdata_dir = self.run_dir / "profdata"
        self.exports_dir = self.run_dir / "exports"
        self.report_path = self.run_dir / "coverage-v1.json"
        self.scope: ScopeConfig | None = None
        self.sources: list[SourceEntry] = []
        self.source_by_path: dict[str, SourceEntry] = {}
        self.initial_source_identity = ""
        self.initial_worktree_identity = ""
        self.initial_git_commit = ""
        self.control_hashes: dict[str, str] = {}
        self.environment: dict[str, str] = {}
        self.tools: dict[str, str] = {}
        self.phases: dict[str, dict[str, Any]] = {}
        self.objects: list[dict[str, Any]] = []
        self.test_reports: list[dict[str, Any]] = []
        self.workload_failed = False
        self.peak_bytes = 0
        self.diff_context: dict[str, Any] | None = None
        self.document: dict[str, Any] = {}
        self.build_timeout = _configured_int("COVERAGE_BUILD_TIMEOUT_SECONDS", 3600)
        self.godot_timeout = _configured_int("COVERAGE_GODOT_TIMEOUT_SECONDS", 1800)
        self.quit_after = _configured_int("COVERAGE_GODOT_QUIT_AFTER", 5000)
        self.runs_to_keep = _configured_int("COVERAGE_RUNS_TO_KEEP", 3)

    def initialize(self) -> None:
        self.scope = load_scope_config(SCOPE_PATH, REPOSITORY)
        self.sources = inventory_sources(REPOSITORY, self.scope)
        self.source_by_path = {entry.path: entry for entry in self.sources}
        inline = inline_test_modules(REPOSITORY, self.sources)
        if inline:
            raise CoverageInfrastructureError(
                f"inline test modules remain in production scope: {', '.join(inline)}",
                "scope",
            )
        self.initial_source_identity = source_identity(self.sources)
        self.initial_worktree_identity, git_status = _worktree_identity(
            self.initial_source_identity
        )
        self.control_hashes = {
            path.as_posix(): sha256_file(REPOSITORY / path) for path in CONTROL_PATHS
        }
        self.run_dir.mkdir(parents=True, exist_ok=False)
        self.logs_dir.mkdir()
        free_before = free_disk(COVERAGE_ROOT)
        git_commit = _git_text("rev-parse", "HEAD").strip()
        self.initial_git_commit = git_commit
        package_records = [
            {"name": package.name, "root": package.root} for package in self.scope.packages
        ]
        exclusion_records = [
            {"pattern": exclusion.pattern, "category": exclusion.category}
            for exclusion in self.scope.exclusions
        ]
        included = sum(entry.classification == "included" for entry in self.sources)
        excluded = len(self.sources) - included
        source_lines = sum(entry.source_lines for entry in self.sources)
        self.document = {
            "$schema": "coverage-v1.schema.json",
            "schema_version": 1,
            "run_id": self.run_id,
            "mode": self.mode,
            "complete": False,
            "outcome": "incomplete",
            "generated_at": None,
            "environment": {
                "git_commit": git_commit,
                "git_dirty": bool(git_status),
                "dirty_diff_sha256": self.initial_worktree_identity,
                "source_identity_sha256": self.initial_source_identity,
                "os": platform.system().lower(),
                "arch": platform.machine(),
                "target_triple": "unavailable",
                "rustc": "unavailable",
                "rustc_llvm_version": "unavailable",
                "llvm_cov": "unavailable",
                "llvm_profdata": "unavailable",
                "cargo_llvm_cov": "0.9.0",
                "godot": "unavailable",
                "cargo_profile": "coverage",
                "features": list(ITEST_FEATURES),
                "cfg_variant": ["coverage"],
                "platform_local": True,
            },
            "disclosure": {
                "instrumented": True,
                "language_scope": ["rust"],
                "godot_host_instrumented": False,
                "gdscript_instrumented": False,
                "benchmark_compatible": False,
                "timing_valid": False,
            },
            "scope": {
                "version": 1,
                "config_path": _relative(SCOPE_PATH),
                "config_sha256": sha256_file(SCOPE_PATH),
                "packages": package_records,
                "exclusions": exclusion_records,
                "files": [entry.to_scope_record(None) for entry in self.sources],
                "summary": {
                    "all_rust_files": len(self.sources),
                    "included": included,
                    "excluded": excluded,
                    "mapped": 0,
                    "unmapped": 0,
                    "source_lines": source_lines,
                },
            },
            "phases": [],
            "objects": {"path": None, "sha256": None, "count": 0, "records": []},
            "test_reports": [],
            "sources": list(COVERAGE_SOURCES),
            "files": [],
            "totals": {
                name: empty_coverage_counts()
                for name in ("merged", "unit-runtime", "test-build", "itest-runtime")
            },
            "diff": None,
            "witnesses": [],
            "artifacts": [],
            "operations": {
                "build_directory": _relative(BUILD_ROOT),
                "run_directory": _relative(self.run_dir),
                "free_disk_before": free_before,
                "free_disk_after": free_before,
                "peak_coverage_bytes": 0,
                "keep_raw": self.keep_raw,
                "raw_pruned": False,
                "failure_evidence_kept": False,
                "runs_kept": self.runs_to_keep,
            },
            "rate_gates": [],
            "errors": [],
            "metadata": {
                "sccache_recache": True,
                "control_files": [
                    {"path": path, "sha256": digest}
                    for path, digest in sorted(self.control_hashes.items())
                ]
            },
        }
        if self.mode == "diff":
            self.document["diff"] = {
                "base": self.base or "unavailable",
                "merge_base": "unavailable",
                "sha256": "sha256:" + "0" * 64,
                "processes": 3,
                "union_reported": True,
                "intersection_gated": True,
                "lines": [],
                "state_counts": {state: 0 for state in state_counts([])},
            }
            self._prepare_diff()
        self._measure_peak()

    def _prepare_diff(self) -> None:
        if self.base is None:
            raise CoverageInfrastructureError("coverage diff requires --base", "diff")
        if not self.base or self.base.startswith("-"):
            raise CoverageInfrastructureError("coverage diff base is invalid", "diff")
        merge_base = _git_text("merge-base", self.base, "HEAD").strip()
        if not merge_base:
            raise CoverageInfrastructureError("git merge-base returned no commit", "diff")
        untracked = [
            path.decode("utf-8")
            for path in _git_bytes("ls-files", "--others", "--exclude-standard", "-z").split(b"\0")
            if path
        ]
        assert self.scope is not None
        unsafe = [
            path
            for path in untracked
            if path.endswith(".rs")
            and self.scope.package_for(path) is not None
            and not self.scope.matching_exclusions(path)
        ]
        if unsafe:
            raise CoverageInfrastructureError(
                f"untracked in-scope Rust files: {', '.join(sorted(unsafe))}", "diff"
            )
        binary = _git_bytes(
            "-c",
            "core.quotePath=false",
            "diff",
            "--no-ext-diff",
            "--binary",
            merge_base,
            "--",
        )
        unified = _git_text(
            "-c",
            "core.quotePath=false",
            "diff",
            "--no-ext-diff",
            "--no-color",
            "--unified=0",
            merge_base,
            "--",
        )
        self.diff_context = {
            "base": self.base,
            "merge_base": merge_base,
            "sha256": sha256_bytes(binary),
            "unified": unified,
        }
        self.document["diff"].update(
            {
                "base": self.base,
                "merge_base": merge_base,
                "sha256": sha256_bytes(binary),
            }
        )

    def _measure_peak(self) -> None:
        self.peak_bytes = max(self.peak_bytes, directory_size(COVERAGE_ROOT))

    def _phase(self, phase_id: str) -> dict[str, Any]:
        if phase_id not in self.phases:
            definition = next(item for item in PHASE_DEFINITIONS if item[0] == phase_id)
            self.phases[phase_id] = {
                "id": phase_id,
                "included": definition[1],
                "source": definition[2],
                "elapsed_seconds": 0.0,
                "output_bytes": 0,
                "raw_files": 0,
                "processes": [],
                "metadata": {},
            }
        return self.phases[phase_id]

    def prepare_tools(self) -> None:
        print("coverage: resolving instrumented tool environment")
        self.environment, self.tools = resolve_tools()
        self.document["environment"].update(
            {
                "target_triple": self.tools["host"],
                "rustc": self.tools["rustc"],
                "rustc_llvm_version": self.tools["rustc_llvm"],
                "llvm_cov": self.tools["llvm_cov_version"],
                "llvm_profdata": self.tools["llvm_profdata_version"],
            }
        )
        stdout = self.logs_dir / "coverage-clean.stdout.log"
        stderr = self.logs_dir / "coverage-clean.stderr.log"
        record = _process(
            ["cargo", "llvm-cov", "clean", "--workspace"],
            self.environment,
            stdout,
            stderr,
            self.build_timeout,
        )
        if record["exit_code"] != 0:
            raise CoverageInfrastructureError("cargo llvm-cov clean failed", "tools")
        self._measure_peak()

    def _run_build(self, phase_id: str, arguments: list[str]) -> tuple[list[Any], Path]:
        phase = self._phase(phase_id)
        raw = _fresh_directory(self.raw_dir / phase_id)
        environment = _phase_environment(self.environment, raw)
        stdout = self.logs_dir / f"{phase_id}.cargo.jsonl"
        stderr = self.logs_dir / f"{phase_id}.stderr.log"
        print(f"coverage: {phase_id}")
        record = _process(arguments, environment, stdout, stderr, self.build_timeout)
        phase["elapsed_seconds"] += record["elapsed_seconds"]
        phase["output_bytes"] += record["output_bytes"]
        phase["processes"].append(
            {
                "id": phase_id,
                "pid": record["pid"],
                "exit_code": record["exit_code"],
                "expected": True,
                "elapsed_seconds": record["elapsed_seconds"],
                "raw_files": len(list(raw.rglob("*.profraw"))),
                "report": None,
                "sentinel": None,
                "metadata": {"command": arguments},
            }
        )
        if record["exit_code"] != 0:
            raise CoverageInfrastructureError(f"{phase_id} Cargo build failed", phase_id)
        try:
            messages = parse_cargo_json(stdout.read_text(encoding="utf-8"))
        except (OSError, CoverageModelError) as error:
            raise CoverageInfrastructureError(str(error), phase_id) from error
        profiles = sorted(raw.rglob("*.profraw"))
        for profile in profiles:
            if profile.stat().st_size == 0:
                raise CoverageInfrastructureError(f"empty build profile: {profile}", phase_id)
        phase["raw_files"] = len(profiles)
        self._measure_peak()
        return messages, stdout

    def _force_rebuild_scope(self, package_names: tuple[str, ...]) -> None:
        # target/coverage/build is reused across runs for dependency artifacts,
        # but in-scope packages must actually recompile: test-build evidence
        # (proc-macro execution) only exists for freshly built objects, and the
        # manifest rejects cargo-fresh in-scope artifacts.
        arguments = ["cargo", "clean", "--profile", "coverage"]
        for package in package_names:
            arguments.extend(["--package", package])
        record = _process(
            arguments,
            self.environment,
            self.logs_dir / "scope-clean.stdout.log",
            self.logs_dir / "scope-clean.stderr.log",
            self.build_timeout,
        )
        if record["exit_code"] != 0:
            raise CoverageInfrastructureError("scope clean failed", "unit-build")

    def build_unit_tests(self) -> tuple[list[Any], list[dict[str, Any]], Path]:
        assert self.scope is not None
        package_names = tuple(package.name for package in self.scope.packages)
        self._force_rebuild_scope(package_names + ("godot-bevy-itest",))
        arguments = [
            "cargo",
            "test",
            "--locked",
            "--profile",
            "coverage",
            "--lib",
            "--no-run",
            "--message-format=json-render-diagnostics",
        ]
        for package in package_names:
            arguments.extend(("-p", package))
        messages, log = self._run_build("unit-build", arguments)
        manifests = {
            package.name: str((REPOSITORY / package.root).parent / "Cargo.toml")
            for package in self.scope.packages
        }
        executable_by_package: dict[str, dict[str, Any]] = {}
        normalized_manifests = {Path(path).resolve(): name for name, path in manifests.items()}
        for message in messages:
            package = normalized_manifests.get(Path(message.manifest_path).resolve())
            if package is None or message.profile["test"] is not True or message.executable is None:
                continue
            if package in executable_by_package:
                raise CoverageInfrastructureError(
                    f"multiple unit executables for {package}", "unit-build"
                )
            executable_by_package[package] = {
                "package": package,
                "path": Path(message.executable),
                "sha256": sha256_file(Path(message.executable)),
            }
        if set(executable_by_package) != set(package_names):
            missing = sorted(set(package_names) - set(executable_by_package))
            raise CoverageInfrastructureError(
                f"unit build lacks exact libtest executables: {', '.join(missing)}",
                "unit-build",
            )
        return messages, [executable_by_package[name] for name in package_names], log

    def run_unit_tests(self, executables: list[dict[str, Any]]) -> None:
        assert self.scope is not None
        phase = self._phase("unit-runtime")
        phase_started = time.monotonic()
        for item in executables:
            package = item["package"]
            executable = item["path"].resolve()
            if not executable.is_file() or sha256_file(executable) != item["sha256"]:
                raise CoverageInfrastructureError(
                    f"Cargo-reported libtest executable identity is invalid: {executable}",
                    "unit-runtime",
                )
            process_raw = _fresh_directory(self.raw_dir / "unit-runtime" / package)
            environment = _phase_environment(self.environment, process_raw)
            # proc-macro libtest binaries link libstd dynamically via @rpath;
            # cargo injects the loader path when it runs them, so we must too.
            loader_var = "DYLD_LIBRARY_PATH" if sys.platform == "darwin" else "LD_LIBRARY_PATH"
            sysroot = Path(self.tools["sysroot"])
            loader_paths = [
                str(sysroot / "lib" / "rustlib" / self.tools["host"] / "lib"),
                str(sysroot / "lib"),
            ]
            existing = environment.get(loader_var, "")
            if existing:
                loader_paths.append(existing)
            environment[loader_var] = ":".join(loader_paths)
            stdout = self.logs_dir / f"unit-runtime-{package}.stdout.log"
            stderr = self.logs_dir / f"unit-runtime-{package}.stderr.log"
            package_root = next(
                item for item in self.scope.packages if item.name == package
            )
            working_directory = (REPOSITORY / package_root.root).parent
            print(f"coverage: unit-runtime {package}")
            record = _process(
                [str(executable)],
                environment,
                stdout,
                stderr,
                self.build_timeout,
                working_directory,
            )
            profiles = discover_profraw(process_raw, record["pid"])
            phase["processes"].append(
                {
                    "id": package,
                    "pid": record["pid"],
                    "exit_code": record["exit_code"],
                    "expected": True,
                    "elapsed_seconds": record["elapsed_seconds"],
                    "raw_files": len(profiles),
                    "report": None,
                    "sentinel": None,
                    "metadata": {
                        "executable": _relative(executable),
                        "working_directory": _relative(working_directory),
                    },
                }
            )
            phase["output_bytes"] += record["output_bytes"]
            if record["exit_code"] != 0:
                self.workload_failed = True
        phase["elapsed_seconds"] = time.monotonic() - phase_started
        phase["raw_files"] = len(list((self.raw_dir / "unit-runtime").rglob("*.profraw")))
        self._measure_peak()

    def build_itest(self) -> tuple[list[Any], Path]:
        arguments = [
            "cargo",
            "build",
            "--locked",
            "--profile",
            "coverage",
            "--lib",
            "--message-format=json-render-diagnostics",
            "-p",
            "godot-bevy-itest",
            "--features",
            ",".join(ITEST_FEATURES),
        ]
        return self._run_build("itest-build", arguments)

    def finalize_objects(
        self,
        unit_messages: list[Any],
        itest_messages: list[Any],
        unit_log: Path,
        itest_log: Path,
    ) -> Path:
        assert self.scope is not None
        manifests = {
            package.name: str((REPOSITORY / package.root).parent / "Cargo.toml")
            for package in self.scope.packages
        }
        try:
            records = select_cargo_objects(
                unit_messages,
                itest_messages,
                manifests,
                str(REPOSITORY / "itest" / "rust" / "Cargo.toml"),
            )
        except CoverageModelError as error:
            raise CoverageInfrastructureError(str(error), "objects") from error
        normalized: list[dict[str, Any]] = []
        for record in records:
            object_path = Path(record["path"]).resolve()
            try:
                object_path.relative_to(BUILD_ROOT.resolve())
            except ValueError as error:
                raise CoverageInfrastructureError(
                    f"Cargo object is outside the coverage build: {object_path}", "objects"
                ) from error
            if not object_path.is_file():
                raise CoverageInfrastructureError(
                    f"Cargo object is missing: {object_path}", "objects"
                )
            normalized.append(
                {
                    **record,
                    "path": _relative(object_path),
                    "sha256": sha256_file(object_path),
                    "target_triple": self.tools["host"],
                    "included": True,
                }
            )
        kind_counts = {
            kind: sum(record["kind"] == kind for record in normalized)
            for kind in ("libtest", "proc-macro", "cdylib")
        }
        # proc-macro dylibs: one per macro crate from the unit graph, plus up to
        # one more per crate when the itest graph unifies features differently.
        if (
            kind_counts["libtest"] != 4
            or kind_counts["cdylib"] != 1
            or not 2 <= kind_counts["proc-macro"] <= 4
        ):
            raise CoverageInfrastructureError(
                f"object manifest has an invalid exact census: {kind_counts}", "objects"
            )
        cdylib = next(record for record in normalized if record["kind"] == "cdylib")
        # cargo lists the (empty) `default` feature alongside the explicit set
        if set(cdylib["features"]) - {"default"} != set(ITEST_FEATURES):
            raise CoverageInfrastructureError(
                f"itest cdylib has unexpected features: {cdylib['features']}", "objects"
            )
        self.objects = normalized
        manifest = {
            "schema_version": 1,
            "target_triple": self.tools["host"],
            "cargo_json": [
                {
                    "phase": "unit-build",
                    "path": _relative(unit_log),
                    "sha256": sha256_file(unit_log),
                },
                {
                    "phase": "itest-build",
                    "path": _relative(itest_log),
                    "sha256": sha256_file(itest_log),
                },
            ],
            "objects": normalized,
        }
        path = self.run_dir / "objects-v1.json"
        write_json(path, manifest)
        self.document["objects"] = {
            "path": _relative(path),
            "sha256": sha256_file(path),
            "count": len(normalized),
            "records": normalized,
        }
        self._measure_peak()
        return path

    def _object_paths(self) -> list[Path]:
        paths: list[Path] = []
        for record in self.objects:
            path = (REPOSITORY / record["path"]).resolve()
            try:
                path.relative_to(BUILD_ROOT.resolve())
            except ValueError as error:
                raise CoverageInfrastructureError(
                    f"coverage object escaped the build root: {record['path']}",
                    "identity",
                ) from error
            if not path.is_file() or sha256_file(path) != record["sha256"]:
                raise CoverageInfrastructureError(
                    f"coverage object identity drifted: {record['path']}", "identity"
                )
            paths.append(path)
        if not paths:
            raise CoverageInfrastructureError("coverage object manifest is empty", "objects")
        return paths

    def _cdylib_path(self) -> Path:
        records = [record for record in self.objects if record["kind"] == "cdylib"]
        if len(records) != 1:
            raise CoverageInfrastructureError("object manifest lacks one itest cdylib", "objects")
        record = records[0]
        if (
            record.get("package") != "godot-bevy-itest"
            or record.get("phase") != "itest-build"
        ):
            raise CoverageInfrastructureError(
                "object manifest identifies the wrong dylib", "objects"
            )
        path = (REPOSITORY / record["path"]).resolve()
        try:
            path.relative_to(BUILD_ROOT.resolve())
        except ValueError as error:
            raise CoverageInfrastructureError(
                "itest cdylib escaped the coverage build", "identity"
            ) from error
        if (
            path.suffix not in {".so", ".dylib"}
            or not path.is_file()
            or sha256_file(path) != record["sha256"]
        ):
            raise CoverageInfrastructureError("itest cdylib identity is invalid", "identity")
        return path

    def _godot_binary(self) -> Path:
        configured = os.environ.get("GODOT4_BIN")
        candidates: list[str] = []
        if configured:
            candidates.append(configured)
        candidates.extend(("godot4", "godot"))
        for candidate in candidates:
            resolved = shutil.which(candidate)
            if resolved:
                return Path(resolved).resolve()
            path = Path(candidate)
            if path.is_absolute() and path.is_file():
                return path.resolve()
        application = Path("/Applications/Godot.app/Contents/MacOS/Godot")
        if application.is_file():
            return application.resolve()
        raise CoverageInfrastructureError("could not find Godot 4; set GODOT4_BIN", "godot")

    @contextmanager
    def _coverage_gdextension(self, cdylib: Path) -> Iterator[None]:
        originals: dict[Path, bytes | None] = {}
        for path in (GDEXTENSION_PATH, EXTENSION_LIST_PATH):
            try:
                path.parent.resolve().relative_to(GODOT_PROJECT.resolve())
            except ValueError as error:
                raise CoverageInfrastructureError(
                    f"unsafe Godot coverage path: {path}", "godot"
                ) from error
            if path.is_symlink():
                raise CoverageInfrastructureError(
                    f"Godot coverage path is a symlink: {path}", "godot"
                )
            originals[path] = path.read_bytes() if path.exists() else None
        relative_library = os.path.relpath(cdylib, GODOT_PROJECT).replace(os.sep, "/")
        extension = (
            "[configuration]\n"
            'entry_symbol = "godot_bevy_itest"\n'
            "compatibility_minimum = 4.2\n\n"
            "[libraries]\n"
            f'linux.debug.x86_64 = "res://{relative_library}"\n'
            f'linux.release.x86_64 = "res://{relative_library}"\n'
            f'macos.debug = "res://{relative_library}"\n'
            f'macos.release = "res://{relative_library}"\n'
            f'macos.debug.arm64 = "res://{relative_library}"\n'
            f'macos.release.arm64 = "res://{relative_library}"\n'
        ).encode()
        try:
            _write_bytes_atomic(GDEXTENSION_PATH, extension)
            _write_bytes_atomic(EXTENSION_LIST_PATH, b"res://itest.gdextension\n")
            yield
        finally:
            restore_error: BaseException | None = None
            for path, original in originals.items():
                try:
                    if original is None:
                        path.unlink(missing_ok=True)
                    else:
                        _write_bytes_atomic(path, original)
                except BaseException as error:
                    if restore_error is None:
                        restore_error = error
            if restore_error is not None:
                raise infrastructure_error(restore_error, "godot")

    def _validate_godot_process(
        self,
        phase_id: str,
        process_id: str,
        raw_directory: Path,
        sentinel: Path,
        record: dict[str, Any],
        report: Path | None,
        exit_file: Path | None,
    ) -> tuple[list[Path], int | None]:
        profiles = discover_profraw(raw_directory, record["pid"])
        try:
            parse_flush_sentinel(sentinel, record["pid"])
        except CoverageModelError as error:
            raise CoverageInfrastructureError(str(error), phase_id) from error
        if record["exit_code"] != 0:
            raise CoverageInfrastructureError(
                f"Godot {process_id} exited with {record['exit_code']}", phase_id
            )
        handshake: int | None = None
        if exit_file is not None:
            try:
                raw_exit = exit_file.read_text(encoding="utf-8").strip()
            except OSError as error:
                raise CoverageInfrastructureError(
                    f"missing Godot test exit handshake: {exit_file}", phase_id
                ) from error
            if raw_exit not in {"0", "1", "2"}:
                raise CoverageInfrastructureError(
                    f"invalid Godot test exit handshake: {raw_exit!r}", phase_id
                )
            handshake = int(raw_exit)
            if handshake == 2:
                raise CoverageInfrastructureError(
                    "Tier-1 harness reported a configuration or infrastructure error", phase_id
                )
        phase = self._phase(phase_id)
        phase["processes"].append(
            {
                "id": process_id,
                "pid": record["pid"],
                "exit_code": record["exit_code"],
                "expected": True,
                "elapsed_seconds": record["elapsed_seconds"],
                "raw_files": len(profiles),
                "report": _relative(report) if report is not None else None,
                "sentinel": _relative(sentinel),
                "metadata": {"test_exit_code": handshake},
            }
        )
        phase["elapsed_seconds"] += record["elapsed_seconds"]
        phase["output_bytes"] += record["output_bytes"]
        phase["raw_files"] += len(profiles)
        return profiles, handshake

    def _reject_flush_errors(self, stderr: Path, phase: str) -> None:
        try:
            output = stderr.read_text(encoding="utf-8", errors="replace")
        except OSError as error:
            raise CoverageInfrastructureError(str(error), phase) from error
        if "coverage flush failed:" in output:
            raise CoverageInfrastructureError(
                "coverage flush attempted a duplicate or could not write its sentinel",
                phase,
            )

    def run_godot(self) -> None:
        godot = self._godot_binary()
        version = _run_text([str(godot), "--version"], self.environment).strip()
        if not version or re.match(r"^4(?:\.|$)", version) is None:
            raise CoverageInfrastructureError(
                f"expected Godot 4, got {version!r}", "godot"
            )
        self.document["environment"]["godot"] = version.splitlines()[0]
        cdylib = self._cdylib_path()
        runtime_count = 3 if self.mode == "diff" else 1
        with self._coverage_gdextension(cdylib):
            import_raw = _fresh_directory(self.raw_dir / "import")
            import_sentinel = self.run_dir / "import-flush-v1.json"
            import_environment = _phase_environment(self.environment, import_raw)
            import_environment["ITEST_COVERAGE_FLUSH_PATH"] = str(import_sentinel.resolve())
            for name in (
                "ITEST_FILTER",
                "ITEST_JSON_PATH",
                "ITEST_REPEAT",
                "GODOT_TEST_EXIT_CODE_PATH",
            ):
                import_environment.pop(name, None)
            print("coverage: Godot import (excluded)")
            import_stderr = self.logs_dir / "import.stderr.log"
            import_record = _process(
                [str(godot), "--headless", "--path", str(GODOT_PROJECT), "--import", "--quit"],
                import_environment,
                self.logs_dir / "import.stdout.log",
                import_stderr,
                self.godot_timeout,
            )
            self._reject_flush_errors(import_stderr, "import")
            self._validate_godot_process(
                "import",
                "import",
                import_raw,
                import_sentinel,
                import_record,
                None,
                None,
            )

            for run in range(1, runtime_count + 1):
                process_raw = _fresh_directory(self.raw_dir / "itest-runtime" / f"run-{run}")
                sentinel = self.run_dir / f"itest-run-{run}-flush-v1.json"
                report = self.run_dir / f"itest-run-{run}.json"
                exit_file = self.run_dir / f"itest-run-{run}.exit"
                environment = _phase_environment(self.environment, process_raw)
                environment.update(
                    {
                        "ITEST_COVERAGE_FLUSH_PATH": str(sentinel.resolve()),
                        "ITEST_JSON_PATH": str(report.resolve()),
                        "GODOT_TEST_EXIT_CODE_PATH": str(exit_file.resolve()),
                        "ITEST_REPEAT": "1",
                        "ITEST_TIMEOUT_FRAMES": "600",
                        "ITEST_BUILD_PROFILE": "debug",
                        "ITEST_DENY_FOCUS": "1",
                    }
                )
                for name in ("ITEST_FILTER", "ITEST_FOCUS"):
                    environment.pop(name, None)
                print(f"coverage: Godot itest-runtime {run}/{runtime_count}")
                runtime_stderr = self.logs_dir / f"itest-runtime-{run}.stderr.log"
                record = _process(
                    [
                        str(godot),
                        "--headless",
                        "--fixed-fps",
                        "60",
                        "--path",
                        str(GODOT_PROJECT),
                        "--quit-after",
                        str(self.quit_after),
                    ],
                    environment,
                    self.logs_dir / f"itest-runtime-{run}.stdout.log",
                    runtime_stderr,
                    self.godot_timeout,
                )
                self._reject_flush_errors(runtime_stderr, "itest-runtime")
                _, handshake = self._validate_godot_process(
                    "itest-runtime",
                    f"itest-run-{run}",
                    process_raw,
                    sentinel,
                    record,
                    report,
                    exit_file,
                )
                try:
                    normalized, passed = parse_itest_report(report, ITEST_SCHEMA_PATH)
                except CoverageModelError as error:
                    raise CoverageInfrastructureError(str(error), "itest-runtime") from error
                expected_handshake = 0 if passed else 1
                if handshake != expected_handshake:
                    raise CoverageInfrastructureError(
                        "Tier-1 report outcome conflicts with the exit handshake",
                        "itest-runtime",
                    )
                if not passed:
                    self.workload_failed = True
                self.test_reports.append(
                    {
                        "run": run,
                        "path": _relative(report),
                        "sha256": sha256_file(report),
                        **normalized,
                    }
                )
        self.document["test_reports"] = self.test_reports
        self._measure_peak()

    def _raw_files(self, path: Path, allow_empty: bool = False) -> list[Path]:
        profiles = sorted(path.rglob("*.profraw")) if path.is_dir() else []
        if not profiles and not allow_empty:
            raise CoverageInfrastructureError(f"no profiles in {path}", "merge")
        for profile in profiles:
            if not profile.is_file() or profile.stat().st_size == 0:
                raise CoverageInfrastructureError(f"invalid raw profile: {profile}", "merge")
            try:
                profile.resolve().relative_to(self.raw_dir.resolve())
            except ValueError as error:
                raise CoverageInfrastructureError(
                    f"raw profile is not phase-owned: {profile}", "merge"
                ) from error
        return profiles

    def _merge_profiles(self, name: str, profiles: list[Path]) -> Path:
        if not profiles:
            raise CoverageInfrastructureError(f"profile group is empty: {name}", "merge")
        output = self.profdata_dir / f"{name}.profdata"
        stdout = self.logs_dir / f"merge-{name}.stdout.log"
        stderr = self.logs_dir / f"merge-{name}.stderr.log"
        arguments = [
            self.tools["llvm_profdata"],
            "merge",
            "--sparse",
            "--failure-mode=any",
            "-o",
            str(output),
            *[str(path) for path in profiles],
        ]
        record = _process(
            arguments, self.environment, stdout, stderr, self.build_timeout
        )
        if record["exit_code"] != 0 or not output.is_file() or output.stat().st_size == 0:
            raise CoverageInfrastructureError(
                f"llvm-profdata rejected the {name} profile group", "merge"
            )
        return output

    def _llvm_cov_arguments(self, profile: Path, output_format: str) -> list[str]:
        objects = self._object_paths()
        included_sources = [
            str((REPOSITORY / source.path).resolve())
            for source in self.sources
            if source.classification == "included"
        ]
        arguments = [
            self.tools["llvm_cov"],
            "export",
            f"-format={output_format}",
            f"-instr-profile={profile}",
            "-check-binary-ids",
            "-debuginfod=false",
        ]
        if self.tools["host"] == "aarch64-apple-darwin":
            arguments.append("-arch=arm64")
        arguments.append(str(objects[0]))
        arguments.extend(f"-object={path}" for path in objects[1:])
        arguments.append("-sources")
        arguments.extend(included_sources)
        return arguments

    def _gzip_export(self, source: Path, destination: Path) -> None:
        destination.parent.mkdir(parents=True, exist_ok=True)
        temporary = destination.with_name(f".{destination.name}.{uuid.uuid4().hex}.tmp")
        try:
            with source.open("rb") as input_file, temporary.open("wb") as raw_output:
                with gzip.GzipFile(fileobj=raw_output, mode="wb", mtime=0) as output:
                    shutil.copyfileobj(input_file, output)
                raw_output.flush()
                os.fsync(raw_output.fileno())
            os.replace(temporary, destination)
        except OSError as error:
            temporary.unlink(missing_ok=True)
            raise infrastructure_error(error, "export") from error

    def _export_json(self, name: str, profile: Path) -> tuple[CoverageIndex, Path]:
        plain = self.exports_dir / f"{name}.json"
        stderr = self.logs_dir / f"export-{name}.stderr.log"
        arguments = self._llvm_cov_arguments(profile, "text")
        record = _process(
            arguments,
            self.environment,
            plain,
            stderr,
            self.build_timeout,
        )
        if record["exit_code"] != 0 or not plain.is_file() or plain.stat().st_size == 0:
            raise CoverageInfrastructureError(f"llvm-cov export failed for {name}", "export")
        self._measure_peak()
        try:
            index = parse_llvm_cov_export(plain.read_text(encoding="utf-8"), REPOSITORY)
        except (OSError, CoverageModelError) as error:
            raise CoverageInfrastructureError(str(error), "export") from error
        destination = (
            self.run_dir / "llvm-cov.json.gz"
            if name == "merged"
            else self.exports_dir / f"{name}.json.gz"
        )
        self._gzip_export(plain, destination)
        plain.unlink()
        return index, destination

    def _export_lcov(self, profile: Path) -> Path:
        output = self.run_dir / "lcov.info"
        stderr = self.logs_dir / "export-lcov.stderr.log"
        record = _process(
            self._llvm_cov_arguments(profile, "lcov"),
            self.environment,
            output,
            stderr,
            self.build_timeout,
        )
        if record["exit_code"] != 0 or not output.is_file() or output.stat().st_size == 0:
            raise CoverageInfrastructureError("llvm-cov LCOV export failed", "export")
        self._measure_peak()
        try:
            text = output.read_text(encoding="utf-8")
        except OSError as error:
            raise CoverageInfrastructureError(str(error), "export") from error
        if "SF:" not in text or "end_of_record" not in text:
            raise CoverageInfrastructureError("llvm-cov emitted malformed LCOV", "export")
        return output

    def generate_exports(self) -> tuple[dict[str, CoverageIndex], list[CoverageIndex], list[Path]]:
        self.profdata_dir.mkdir()
        self.exports_dir.mkdir()
        unit_build = self._raw_files(self.raw_dir / "unit-build", allow_empty=True)
        itest_build = self._raw_files(self.raw_dir / "itest-build", allow_empty=True)
        test_build = unit_build + itest_build
        if not test_build:
            raise CoverageInfrastructureError(
                "clean construction emitted no proc-macro build profiles", "merge"
            )
        unit_runtime = self._raw_files(self.raw_dir / "unit-runtime")
        import_profiles = self._raw_files(self.raw_dir / "import")
        runtime_count = 3 if self.mode == "diff" else 1
        itest_groups = [
            self._raw_files(self.raw_dir / "itest-runtime" / f"run-{run}")
            for run in range(1, runtime_count + 1)
        ]
        itest_runtime = [profile for group in itest_groups for profile in group]
        included_profiles = test_build + unit_runtime + itest_runtime
        if set(import_profiles).intersection(included_profiles):
            raise CoverageInfrastructureError(
                "import profiles contaminated included evidence", "merge"
            )

        raw_ledger = {
            "schema_version": 1,
            "profile_pattern": "%p-%m.profraw",
            "failure_mode": "any",
            "records": [],
        }
        groups = {
            "unit-build": (unit_build, True, "test-build"),
            "unit-runtime": (unit_runtime, True, "unit-runtime"),
            "itest-build": (itest_build, True, "test-build"),
            "import": (import_profiles, False, "excluded"),
            "itest-runtime": (itest_runtime, True, "itest-runtime"),
        }
        for phase, (profiles, included, source) in groups.items():
            for profile in profiles:
                raw_ledger["records"].append(
                    {
                        "phase": phase,
                        "source": source,
                        "included": included,
                        "path": _relative(profile),
                        "sha256": sha256_file(profile),
                        "size_bytes": profile.stat().st_size,
                    }
                )
        raw_ledger_path = self.run_dir / "raw-ledger-v1.json"
        write_json(raw_ledger_path, raw_ledger)

        profiles: dict[str, Path] = {
            "test-build": self._merge_profiles("test-build", test_build),
            "unit-runtime": self._merge_profiles("unit-runtime", unit_runtime),
            "itest-runtime": self._merge_profiles("itest-runtime", itest_runtime),
            "merged": self._merge_profiles("merged", included_profiles),
        }
        for index, group in enumerate(itest_groups, 1):
            profiles[f"itest-run-{index}"] = self._merge_profiles(
                f"itest-run-{index}", group
            )

        indexes: dict[str, CoverageIndex] = {}
        export_artifacts: list[Path] = []
        for name, profile in profiles.items():
            print(f"coverage: exporting {name}")
            indexes[name], compressed = self._export_json(name, profile)
            export_artifacts.append(compressed)
        lcov = self._export_lcov(profiles["merged"])
        export_artifacts.extend((lcov, raw_ledger_path))

        itest_indexes = [indexes[f"itest-run-{run}"] for run in range(1, runtime_count + 1)]
        included_paths = [
            source.path for source in self.sources if source.classification == "included"
        ]
        validate_mapping_identity(
            indexes["merged"],
            [
                indexes["unit-runtime"],
                indexes["test-build"],
                indexes["itest-runtime"],
                *itest_indexes,
            ],
            included_paths,
        )
        allowed = set(included_paths)
        for name, index in indexes.items():
            unexpected = sorted(set(index.files) - allowed)
            if unexpected:
                raise CoverageInfrastructureError(
                    f"{name} export escaped positive source scope: {', '.join(unexpected)}",
                    "scope",
                )
        self._measure_peak()
        return indexes, itest_indexes, export_artifacts

    def normalize_reports(
        self,
        indexes: dict[str, CoverageIndex],
        itest_indexes: list[CoverageIndex],
    ) -> Path:
        merged = indexes["merged"]
        unit = indexes["unit-runtime"]
        build = indexes["test-build"]
        itest_union = indexes["itest-runtime"]
        scope_records: list[dict[str, Any]] = []
        file_records: list[dict[str, Any]] = []
        mapped_count = 0
        unmapped_count = 0
        totals_by_source: dict[str, list[dict[str, dict[str, int]]]] = {
            name: [] for name in ("merged", "unit-runtime", "test-build", "itest-runtime")
        }
        for source in self.sources:
            if source.classification == "excluded":
                scope_records.append(source.to_scope_record(None))
                continue
            mapping = "mapped" if source.path in merged.files else "unmapped"
            scope_records.append(source.to_scope_record(mapping))
            if mapping == "mapped":
                mapped_count += 1
            else:
                unmapped_count += 1
            source_indexes = {
                "merged": merged,
                "unit-runtime": unit,
                "test-build": build,
                "itest-runtime": itest_union,
            }
            counts: dict[str, dict[str, dict[str, int]]] = {}
            for name, index in source_indexes.items():
                coverage = index.files.get(source.path)
                value = (
                    coverage.coverage_counts()
                    if coverage is not None
                    else empty_coverage_counts()
                )
                counts[name] = value
                totals_by_source[name].append(value)
            regions: list[dict[str, Any]] = []
            mapped = merged.files.get(source.path)
            if mapped is not None:
                for region in sorted(mapped.regions):
                    regions.append(
                        {
                            "start_line": region.start_line,
                            "start_column": region.start_column,
                            "end_line": region.end_line,
                            "end_column": region.end_column,
                            "kind": region.kind,
                            "merged_count": merged.count(source.path, region),
                            "unit_runtime_count": unit.count(source.path, region),
                            "test_build_count": build.count(source.path, region),
                            "itest_runtime_counts": [
                                index.count(source.path, region) for index in itest_indexes
                            ],
                        }
                    )
            file_records.append(
                {
                    "path": source.path,
                    "package": source.package,
                    "mapping": mapping,
                    "sha256": source.sha256,
                    "source_lines": source.source_lines,
                    "counts": counts,
                    "regions": regions,
                }
            )
        self.document["scope"]["files"] = scope_records
        self.document["scope"]["summary"].update(
            {"mapped": mapped_count, "unmapped": unmapped_count}
        )
        self.document["files"] = file_records
        self.document["totals"] = {
            name: sum_coverage_counts(values) for name, values in totals_by_source.items()
        }
        ledger = {
            "schema_version": 1,
            "scope_config": {
                "path": _relative(SCOPE_PATH),
                "sha256": sha256_file(SCOPE_PATH),
            },
            "packages": self.document["scope"]["packages"],
            "exclusions": self.document["scope"]["exclusions"],
            "summary": self.document["scope"]["summary"],
            "files": scope_records,
        }
        path = self.run_dir / "sources-v1.json"
        write_json(path, ledger)
        return path

    def evaluate_witnesses(
        self,
        indexes: dict[str, CoverageIndex],
        itest_indexes: list[CoverageIndex],
    ) -> None:
        assert self.scope is not None
        try:
            witnesses = load_witnesses(WITNESSES_PATH, REPOSITORY, self.scope)
            records = [
                evaluate_witness(
                    REPOSITORY,
                    witness,
                    indexes["merged"],
                    indexes["unit-runtime"],
                    itest_indexes,
                )
                for witness in witnesses
            ]
        except CoverageModelError as error:
            raise CoverageInfrastructureError(str(error), "witness") from error
        failed = [record["id"] for record in records if not record["passed"]]
        if failed:
            raise CoverageInfrastructureError(
                f"coverage witness failed: {', '.join(failed)}", "witness"
            )
        self.document["witnesses"] = records
        transform = next(
            record for record in records if record["id"] == "post_update_godot_transforms"
        )
        if transform["unit_runtime"] != 0 or not all(
            count > 0 for count in transform["itest_runtime"]
        ):
            raise CoverageInfrastructureError("transform witness attribution is invalid", "witness")
        print(
            "PASS coverage Godot witness: "
            "post_update_godot_transforms unit=0 itest>0"
        )
        isolation = next(record for record in records if record["id"] == "is_reasonable_float")
        if isolation["unit_runtime"] <= 0 or any(
            count != 0 for count in isolation["itest_runtime"]
        ):
            raise CoverageInfrastructureError("unit-only witness attribution is invalid", "witness")
        print("PASS coverage isolation witness: is_reasonable_float unit>0 itest=0")

    def classify_current_diff(
        self,
        indexes: dict[str, CoverageIndex],
        itest_indexes: list[CoverageIndex],
    ) -> tuple[int, str]:
        if self.diff_context is None:
            raise CoverageInfrastructureError("diff context is missing", "diff")
        try:
            changes = parse_unified_diff(self.diff_context["unified"])
            records = classify_diff(
                changes,
                self.source_by_path,
                indexes["merged"],
                indexes["unit-runtime"],
                indexes["test-build"],
                itest_indexes,
            )
        except CoverageModelError as error:
            raise CoverageInfrastructureError(str(error), "diff") from error
        self.document["diff"].update(
            {
                "lines": records,
                "state_counts": state_counts(records),
            }
        )
        return diff_exit(records)

    def verify_identity(self) -> None:
        assert self.scope is not None
        current = inventory_sources(REPOSITORY, self.scope)
        before = [
            (entry.path, entry.classification, entry.sha256) for entry in self.sources
        ]
        after = [(entry.path, entry.classification, entry.sha256) for entry in current]
        if before != after or source_identity(current) != self.initial_source_identity:
            raise CoverageInfrastructureError(
                "coverage source identity drifted during capture", "identity"
            )
        current_worktree, _ = _worktree_identity(source_identity(current))
        if current_worktree != self.initial_worktree_identity:
            raise CoverageInfrastructureError(
                "coverage dirty diff drifted during capture", "identity"
            )
        if _git_text("rev-parse", "HEAD").strip() != self.initial_git_commit:
            raise CoverageInfrastructureError(
                "coverage git commit drifted during capture", "identity"
            )
        for path, digest in self.control_hashes.items():
            if sha256_file(REPOSITORY / path) != digest:
                raise CoverageInfrastructureError(
                    f"coverage control file drifted during capture: {path}", "identity"
                )
        for report in self.test_reports:
            path = REPOSITORY / report["path"]
            if not path.is_file() or sha256_file(path) != report["sha256"]:
                raise CoverageInfrastructureError(
                    f"Tier-1 report identity drifted during capture: {report['path']}",
                    "identity",
                )
        self._object_paths()
        if self.diff_context is not None:
            current_diff = _git_bytes(
                "-c",
                "core.quotePath=false",
                "diff",
                "--no-ext-diff",
                "--binary",
                self.diff_context["merge_base"],
                "--",
            )
            if sha256_bytes(current_diff) != self.diff_context["sha256"]:
                raise CoverageInfrastructureError(
                    "coverage diff identity drifted during capture", "identity"
                )

    def _write_logs_manifest(self) -> Path:
        records = []
        for path in sorted(self.logs_dir.glob("*")):
            if path.is_file():
                records.append(
                    {
                        "path": _relative(path),
                        "sha256": sha256_file(path),
                        "size_bytes": path.stat().st_size,
                    }
                )
        manifest = {"schema_version": 1, "logs": records}
        path = self.run_dir / "logs-v1.json"
        write_json(path, manifest)
        return path

    def _write_process_manifest(self) -> Path:
        path = self.run_dir / "processes-v1.json"
        write_json(
            path,
            {
                "schema_version": 1,
                "profile_pattern": "%p-%m.profraw",
                "phases": [
                    self.phases[phase]
                    for phase, _, _ in PHASE_DEFINITIONS
                    if phase in self.phases
                ],
            },
        )
        return path

    def _collect_artifacts(self, extra: list[Path] | None = None) -> list[dict[str, Any]]:
        candidates: list[tuple[str, Path, dict[str, Any]]] = []
        fixed = (
            ("objects-manifest", self.run_dir / "objects-v1.json"),
            ("source-ledger", self.run_dir / "sources-v1.json"),
            ("raw-ledger", self.run_dir / "raw-ledger-v1.json"),
            ("process-ledger", self.run_dir / "processes-v1.json"),
            ("logs-ledger", self.run_dir / "logs-v1.json"),
            ("llvm-cov-json", self.run_dir / "llvm-cov.json.gz"),
            ("lcov", self.run_dir / "lcov.info"),
            ("import-flush", self.run_dir / "import-flush-v1.json"),
        )
        for kind, path in fixed:
            candidates.append((kind, path, {}))
        for path in sorted(self.exports_dir.glob("*.json.gz")) if self.exports_dir.exists() else []:
            candidates.append((f"export-{path.name.removesuffix('.json.gz')}", path, {}))
        for report in self.test_reports:
            run = report["run"]
            candidates.extend(
                (
                    (f"itest-report-{run}", REPOSITORY / report["path"], {"tier": 1}),
                    (f"itest-flush-{run}", self.run_dir / f"itest-run-{run}-flush-v1.json", {}),
                    (f"itest-exit-{run}", self.run_dir / f"itest-run-{run}.exit", {}),
                )
            )
        if extra:
            known = {path.resolve() for _, path, _ in candidates if path.exists()}
            for index, path in enumerate(extra, 1):
                if path.exists() and path.resolve() not in known:
                    candidates.append((f"evidence-{index}", path, {}))
        artifacts: list[dict[str, Any]] = []
        kinds: set[str] = set()
        for kind, path, metadata in candidates:
            if not path.is_file():
                continue
            if kind in kinds:
                raise CoverageInfrastructureError(f"duplicate artifact kind: {kind}", "report")
            kinds.add(kind)
            artifacts.append(_artifact(path, kind, metadata))
        return artifacts

    def _prune_success_data(self) -> None:
        for path in (self.raw_dir, self.profdata_dir):
            try:
                _remove_path(path)
            except OSError as error:
                raise infrastructure_error(error, "cleanup") from error
        self.document["operations"]["raw_pruned"] = True

    def _prune_old_successes(self) -> None:
        successful: list[Path] = []
        for path in sorted(RUNS_ROOT.iterdir()) if RUNS_ROOT.is_dir() else []:
            if not path.is_dir() or path.is_symlink():
                continue
            report = path / "coverage-v1.json"
            try:
                document = json.loads(report.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError):
                continue
            if document.get("complete") is True and document.get("outcome") in {"pass", "skip"}:
                successful.append(path)
        previous_limit = max(0, self.runs_to_keep - 1)
        for path in successful[: max(0, len(successful) - previous_limit)]:
            if path != self.run_dir:
                _remove_path(path)

    def _finalize_document(
        self,
        final_exit: int,
        outcome: str,
        extra_artifacts: list[Path] | None = None,
    ) -> None:
        self.document["phases"] = [
            self.phases[phase] for phase, _, _ in PHASE_DEFINITIONS if phase in self.phases
        ]
        self.document["test_reports"] = self.test_reports
        self.document["complete"] = final_exit != 2
        self.document["outcome"] = outcome
        self.document["generated_at"] = utc_now()
        self.document["operations"]["failure_evidence_kept"] = final_exit != 0
        self._measure_peak()
        self.document["operations"]["peak_coverage_bytes"] = self.peak_bytes
        if final_exit == 0:
            self._prune_old_successes()
        should_prune = final_exit == 0 and not self.keep_raw
        self.document["operations"]["raw_pruned"] = should_prune
        self.document["operations"]["free_disk_after"] = free_disk(COVERAGE_ROOT)
        self._write_process_manifest()
        self._write_logs_manifest()
        self.document["artifacts"] = self._collect_artifacts(extra_artifacts)
        try:
            validate_coverage_document(self.document, SCHEMA_PATH)
        except CoverageModelError as error:
            raise CoverageInfrastructureError(str(error), "report") from error
        if should_prune:
            self._prune_success_data()
            self.document["operations"]["free_disk_after"] = free_disk(COVERAGE_ROOT)
        write_json(self.report_path, self.document)
        _write_bytes_atomic(LATEST_PATH, (self.run_id + "\n").encode())

    def execute(self) -> int:
        try:
            self.initialize()
        except BaseException as error:
            return self.fail(infrastructure_error(error))
        try:
            self.prepare_tools()
            unit_messages, unit_executables, unit_log = self.build_unit_tests()
            self.run_unit_tests(unit_executables)
            itest_messages, itest_log = self.build_itest()
            objects_path = self.finalize_objects(
                unit_messages, itest_messages, unit_log, itest_log
            )
            self.run_godot()
            indexes, itest_indexes, export_artifacts = self.generate_exports()
            source_ledger = self.normalize_reports(indexes, itest_indexes)
            self.evaluate_witnesses(indexes, itest_indexes)
            self.verify_identity()

            terminal: str | None = None
            if self.mode == "diff":
                verdict, terminal = self.classify_current_diff(indexes, itest_indexes)
                if verdict == 2:
                    self.document["errors"].append(
                        {
                            "kind": "not-mapped",
                            "message": (
                                "changed in-scope source is absent from "
                                "current-platform mappings"
                            ),
                            "phase": "diff",
                        }
                    )
                    final_exit, outcome = 2, "error"
                elif self.workload_failed or verdict == 1:
                    final_exit, outcome = 1, "fail"
                    if self.workload_failed and verdict == 0:
                        terminal = (
                            "FAIL coverage-diff: uncovered, partial, or unstable changed regions"
                        )
                else:
                    final_exit = 0
                    outcome = "skip" if terminal.startswith("SKIP") else "pass"
            elif self.workload_failed:
                final_exit, outcome = 1, "fail"
            else:
                final_exit, outcome = 0, "pass"

            self._finalize_document(
                final_exit,
                outcome,
                [objects_path, source_ledger, *export_artifacts],
            )
            if self.mode == "diff":
                assert terminal is not None
                print(f"Coverage report: {_relative(self.report_path)}")
                print(terminal)
            else:
                if final_exit == 0:
                    print(
                        "PASS coverage: complete "
                        "sources=unit-runtime,test-build,itest-runtime rate-gates=none"
                    )
                else:
                    print("FAIL coverage: unit or integration workload failed")
                print(f"Coverage report: {_relative(self.report_path)}")
            return final_exit
        except BaseException as error:
            if isinstance(error, KeyboardInterrupt):
                normalized = CoverageInfrastructureError("coverage run interrupted", "process")
            else:
                normalized = infrastructure_error(error)
            return self.fail(normalized)

    def fail(self, error: CoverageInfrastructureError) -> int:
        self.document.setdefault("errors", []).append(
            {"kind": "incomplete-evidence", "message": str(error), "phase": error.phase}
        )
        self.document["complete"] = False
        self.document["outcome"] = "error"
        self.document["generated_at"] = utc_now()
        self.document["phases"] = [
            self.phases[phase] for phase, _, _ in PHASE_DEFINITIONS if phase in self.phases
        ]
        self.document["test_reports"] = self.test_reports
        operations = self.document.get("operations")
        if isinstance(operations, dict):
            operations["failure_evidence_kept"] = True
            operations["raw_pruned"] = False
            try:
                self._measure_peak()
                operations["peak_coverage_bytes"] = self.peak_bytes
                operations["free_disk_after"] = free_disk(COVERAGE_ROOT)
            except BaseException:
                pass
        try:
            if self.run_dir.is_dir():
                if self.phases:
                    self._write_process_manifest()
                self._write_logs_manifest()
                self.document["artifacts"] = self._collect_artifacts()
                validate_coverage_document(self.document, SCHEMA_PATH)
                write_json(self.report_path, self.document)
                _write_bytes_atomic(LATEST_PATH, (self.run_id + "\n").encode())
        except BaseException as report_error:
            print(f"ERROR coverage report: {report_error}", file=sys.stderr)
        print(f"ERROR coverage: {error}", file=sys.stderr)
        if self.report_path.is_file():
            print(f"Coverage report: {_relative(self.report_path)}")
        if self.mode == "diff":
            print("ERROR coverage-diff: incomplete evidence")
        return 2


def run_coverage(mode: str, base: str | None, keep_raw: bool) -> int:
    try:
        with CoverageLock():
            run = CoverageRun(mode, base, keep_raw)
            return run.execute()
    except BaseException as error:
        normalized = infrastructure_error(error)
        print(f"ERROR coverage: {normalized}", file=sys.stderr)
        if mode == "diff":
            print("ERROR coverage-diff: incomplete evidence")
        return 2


def _parse_arguments(arguments: list[str]) -> tuple[str, str | None, bool] | None:
    if arguments and arguments[0] == "clean":
        parser = argparse.ArgumentParser(prog="coverage clean")
        parser.parse_args(arguments[1:])
        return None
    if arguments and arguments[0] == "diff":
        parser = argparse.ArgumentParser(prog="coverage diff")
        parser.add_argument("--base", required=True)
        parser.add_argument("--keep-raw", action="store_true")
        parsed = parser.parse_args(arguments[1:])
        return "diff", parsed.base, parsed.keep_raw
    parser = argparse.ArgumentParser(prog="coverage")
    parser.add_argument("--keep-raw", action="store_true")
    parsed = parser.parse_args(arguments)
    return "full", None, parsed.keep_raw


def main() -> int:
    try:
        parsed = _parse_arguments(sys.argv[1:])
    except SystemExit as error:
        if error.code != 0 and sys.argv[1:2] == ["diff"]:
            print("ERROR coverage-diff: incomplete evidence")
        return int(error.code)
    if parsed is None:
        return clean_coverage()
    mode, base, keep_raw = parsed
    return run_coverage(mode, base, keep_raw)


if __name__ == "__main__":
    raise SystemExit(main())
