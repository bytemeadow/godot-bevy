#!/usr/bin/env python3
from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import os
import platform
import re
import signal
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass, replace
from datetime import datetime
from pathlib import Path, PurePosixPath
from typing import Any

from qualification_compare import compare_mutant_outcomes
from qualification_schema import (
    artifact_record,
    error_record,
    load_qualification,
    new_document,
    qualification_exit,
    utc_now,
    write_qualification,
)
from qualification_toml import load_toml

CARGO_MUTANTS_VERSION = "27.1.0"
WALL_CLOCK_SECONDS = int(os.environ.get("QUALIFICATION_WALL_CLOCK", "1800"))
REPOSITORY = Path(__file__).resolve().parents[1]
SCHEMA = REPOSITORY / "godot-bevy-test" / "schema" / "qualification-v1.schema.json"
CONFIG = REPOSITORY / ".cargo" / "mutants.toml"
BASELINE = REPOSITORY / "itest" / "qualification" / "mutants-baseline-v1.json"
WAIVERS = REPOSITORY / "itest" / "qualification" / "mutants-waivers-v1.toml"
OUTPUT_ROOT = REPOSITORY / "target" / "qualification" / "mutants"
LATEST = REPOSITORY / "target" / "qualification" / "latest" / "qualification-v1.json"
OUTCOME_NAMES = {
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Timeout": "timeout",
    "Unviable": "unviable",
}
EXPECTED_SCOPE = [
    "godot-bevy/src/plugins/audio/channel.rs",
    "godot-bevy/src/plugins/event_bridge.rs",
    "godot-bevy/src/plugins/fixed_schedule.rs",
    "godot-bevy/src/plugins/input/actions.rs",
    "godot-bevy/src/plugins/scene_tree/relationship.rs",
    "godot-bevy/src/plugins/transforms/conversions.rs",
    "godot-bevy/src/plugins/transforms/custom_sync.rs",
    "godot-bevy/src/plugins/transforms/math.rs",
    "godot-bevy/src/utils/math.rs",
    "godot-bevy-macros/src/bevy_attr.rs",
    "godot-bevy-macros/src/emit.rs",
    "godot-bevy-test-macros/src/lib.rs",
    "godot-bevy-test/src/config.rs",
    "godot-bevy-test/src/report.rs",
    "godot-bevy-test/src/selection.rs",
]


class MutationEvidenceError(ValueError):
    pass


@dataclass(frozen=True)
class MutationWaiver:
    stable_id: str
    package: str
    path: str
    function: str | None
    genre: str
    replacement: str
    source_slice_sha256: str
    ordinal: int
    rationale: str
    reference: str


@dataclass(frozen=True)
class MutationRecord:
    stable_id: str
    package: str
    path: str
    function: str | None
    genre: str
    replacement: str
    source_slice_sha256: str
    ordinal: int
    source_slice: bytes
    outcome: str
    name: str
    line: int
    column: int
    log_path: str | None
    diff_path: str | None
    metadata: dict[str, Any]

    def as_report_record(self) -> dict[str, Any]:
        return {
            "id": self.stable_id,
            "package": self.package,
            "path": self.path,
            "function": self.function,
            "genre": self.genre,
            "replacement": self.replacement,
            "source_slice_sha256": self.source_slice_sha256,
            "ordinal": self.ordinal,
            "outcome": self.outcome,
            "name": self.name,
            "location": {"line": self.line, "column": self.column},
            "log_path": self.log_path,
            "diff_path": self.diff_path,
            "metadata": self.metadata,
        }


@dataclass(frozen=True)
class MutationRun:
    version: str
    baseline_passed: bool
    start_time: str
    end_time: str
    mutants: list[MutationRecord]


def _object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise MutationEvidenceError(f"{label} must be an object")
    return value


def _string(value: Any, label: str, *, empty: bool = False) -> str:
    if not isinstance(value, str) or (not empty and not value):
        raise MutationEvidenceError(f"{label} must be a nonempty string")
    return value


def _nonnegative_integer(value: Any, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        raise MutationEvidenceError(f"{label} must be a nonnegative integer")
    return value


def _timestamp(value: Any, label: str) -> tuple[str, datetime]:
    raw = _string(value, label)
    try:
        parsed = datetime.fromisoformat(raw.replace("Z", "+00:00"))
    except ValueError as error:
        raise MutationEvidenceError(f"{label} must be an RFC 3339 timestamp") from error
    if parsed.tzinfo is None:
        raise MutationEvidenceError(f"{label} must include a timezone")
    return raw, parsed


def _normalized_relative_path(value: Any, label: str) -> str:
    raw = _string(value, label).replace("\\", "/")
    while raw.startswith("./"):
        raw = raw[2:]
    path = PurePosixPath(raw)
    if path.is_absolute() or not path.parts or any(part == ".." for part in path.parts):
        raise MutationEvidenceError(f"{label} escapes its evidence root: {value!r}")
    return path.as_posix()


def _evidence_path(root: Path, value: Any, label: str) -> tuple[str, Path]:
    relative = _normalized_relative_path(value, label)
    path = (root / relative).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError as error:
        raise MutationEvidenceError(f"{label} escapes its evidence root") from error
    if not path.is_file():
        raise MutationEvidenceError(f"{label} does not exist: {relative}")
    return relative, path


def _position(value: Any, label: str) -> tuple[int, int]:
    position = _object(value, label)
    if set(position) != {"line", "column"}:
        raise MutationEvidenceError(f"{label} has unexpected fields")
    line = position.get("line")
    column = position.get("column")
    if (
        not isinstance(line, int)
        or isinstance(line, bool)
        or line < 1
        or not isinstance(column, int)
        or isinstance(column, bool)
        or column < 1
    ):
        raise MutationEvidenceError(f"{label} has an invalid line/column")
    return line, column


def _phase_results(
    value: Any, summary: str, label: str, *, baseline: bool = False
) -> None:
    if not isinstance(value, list) or not value:
        raise MutationEvidenceError(f"{label}.phase_results must be a nonempty array")
    phases: list[tuple[str, str]] = []
    for index, raw in enumerate(value):
        phase = _object(raw, f"{label}.phase_results[{index}]")
        if set(phase) != {"phase", "duration", "process_status", "argv"}:
            raise MutationEvidenceError(
                f"{label}.phase_results[{index}] has unexpected fields"
            )
        phase_name = _string(phase["phase"], f"{label}.phase_results[{index}].phase")
        if phase_name not in {"Check", "Build", "Test"}:
            raise MutationEvidenceError(f"{label}.phase_results[{index}] has unknown phase")
        duration = phase["duration"]
        if (
            not isinstance(duration, (int, float))
            or isinstance(duration, bool)
            or duration < 0
        ):
            raise MutationEvidenceError(
                f"{label}.phase_results[{index}] has invalid duration"
            )
        argv = phase["argv"]
        if not isinstance(argv, list) or not argv or not all(
            isinstance(argument, str) for argument in argv
        ):
            raise MutationEvidenceError(f"{label}.phase_results[{index}] has invalid argv")
        status = phase["process_status"]
        if isinstance(status, str) and status in {"Success", "Timeout", "Other"}:
            status_name = status
        elif (
            isinstance(status, dict)
            and len(status) == 1
            and next(iter(status)) in {"Failure", "Signalled"}
            and isinstance(next(iter(status.values())), int)
            and not isinstance(next(iter(status.values())), bool)
        ):
            status_name = next(iter(status))
        else:
            raise MutationEvidenceError(
                f"{label}.phase_results[{index}] has invalid process_status"
            )
        phases.append((phase_name, status_name))

    phase_order = {"Check": 0, "Build": 1, "Test": 2}
    order = [phase_order[phase] for phase, _ in phases]
    if order != sorted(set(order)):
        raise MutationEvidenceError(f"{label}.phase_results are out of order")

    last_phase, last_status = phases[-1]
    if baseline:
        if any(status == "Timeout" for _, status in phases):
            expected_summary = "Timeout"
        elif last_status == "Success":
            expected_summary = "Success"
        else:
            expected_summary = "Failure"
    elif any(
        phase != "Test" and status == "Failure" for phase, status in phases
    ):
        expected_summary = "Unviable"
    elif any(status == "Timeout" for _, status in phases):
        expected_summary = "Timeout"
    elif last_phase == "Test" and last_status == "Failure":
        expected_summary = "CaughtMutant"
    elif last_phase == "Test" and last_status == "Success":
        expected_summary = "MissedMutant"
    elif last_status == "Success":
        expected_summary = "Success"
    else:
        expected_summary = "Failure"
    if summary != expected_summary:
        raise MutationEvidenceError(f"{label}.summary conflicts with phase_results")


def _source_slice(source_path: Path, span_value: Any) -> tuple[bytes, int, int]:
    span = _object(span_value, "mutant.span")
    if set(span) != {"start", "end"}:
        raise MutationEvidenceError("mutant.span has unexpected fields")
    start_line, start_column = _position(span.get("start"), "mutant.span.start")
    end_line, end_column = _position(span.get("end"), "mutant.span.end")
    if (end_line, end_column) <= (start_line, start_column):
        raise MutationEvidenceError("mutant source span is empty or reversed")
    try:
        source = source_path.read_bytes().decode("utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise MutationEvidenceError(f"could not read mutant source: {error}") from error

    line = 1
    column = 1
    positions: set[tuple[int, int]] = set()
    extracted: list[str] = []
    for character in source:
        positions.add((line, column))
        if (
            ((line == start_line and column >= start_column) or line > start_line)
            and (line < end_line or (line == end_line and column < end_column))
        ):
            extracted.append(character)
        if character == "\n":
            line += 1
            column = 1
        elif character != "\r":
            column += 1
        if line > end_line or (line == end_line and column >= end_column):
            break
    positions.add((line, column))
    if (start_line, start_column) not in positions or (end_line, end_column) not in positions:
        raise MutationEvidenceError("mutant span position is outside the source file")
    source_slice = "".join(extracted).encode("utf-8")
    if not source_slice:
        raise MutationEvidenceError("mutant source span is empty or reversed")
    return source_slice, start_line, start_column


def _function_name(value: Any) -> str | None:
    if value is None:
        return None
    function = _object(value, "mutant.function")
    if set(function) != {"function_name", "return_type", "span"}:
        raise MutationEvidenceError("mutant.function has unexpected fields")
    _string(function.get("return_type"), "mutant.function.return_type", empty=True)
    function_span = _object(function.get("span"), "mutant.function.span")
    if set(function_span) != {"start", "end"}:
        raise MutationEvidenceError("mutant.function.span has unexpected fields")
    _position(function_span.get("start"), "mutant.function.span.start")
    _position(function_span.get("end"), "mutant.function.span.end")
    return _string(function.get("function_name"), "mutant.function.function_name")


def _stable_id(
    package: str,
    path: str,
    function: str | None,
    genre: str,
    replacement: str,
    source_slice_sha256: str,
    ordinal: int,
) -> str:
    # ordinal disambiguates repeated identical mutations within one function
    # (e.g. two `>` operators); it is the span-order index within the group,
    # so it shifts only when that function's own mutation set changes.
    identity = {
        "function": function,
        "genre": genre,
        "ordinal": ordinal,
        "package": package,
        "path": path,
        "replacement": replacement,
        "source_slice_sha256": source_slice_sha256,
    }
    canonical = json.dumps(
        identity,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return f"sha256:{hashlib.sha256(canonical).hexdigest()}"


def load_waivers(path: Path = WAIVERS) -> dict[str, MutationWaiver]:
    document = load_toml(path)
    if set(document) != {"version", "waivers"} or document.get("version") != 1:
        raise MutationEvidenceError("mutant waiver manifest must be version 1")
    entries = document.get("waivers")
    if not isinstance(entries, list):
        raise MutationEvidenceError("mutant waiver manifest must contain waiver tables")
    required = {
        "id",
        "package",
        "path",
        "function",
        "genre",
        "replacement",
        "source_slice_sha256",
        "ordinal",
        "rationale",
        "reference",
    }
    waivers: dict[str, MutationWaiver] = {}
    for index, raw in enumerate(entries):
        entry = _object(raw, f"waivers[{index}]")
        if set(entry) != required:
            raise MutationEvidenceError(f"waivers[{index}] has unexpected fields")
        stable_id = _string(entry["id"], f"waivers[{index}].id")
        package = _string(entry["package"], f"waivers[{index}].package")
        source_path = _normalized_relative_path(
            entry["path"], f"waivers[{index}].path"
        )
        function_value = entry["function"]
        function = _string(
            function_value, f"waivers[{index}].function", empty=True
        )
        function = function or None
        genre = _string(entry["genre"], f"waivers[{index}].genre")
        replacement = _string(
            entry["replacement"], f"waivers[{index}].replacement", empty=True
        )
        source_hash = _string(
            entry["source_slice_sha256"],
            f"waivers[{index}].source_slice_sha256",
        )
        if re.fullmatch(r"[0-9a-f]{64}", source_hash) is None:
            raise MutationEvidenceError(f"waivers[{index}] has an invalid source hash")
        ordinal = _nonnegative_integer(entry["ordinal"], f"waivers[{index}].ordinal")
        rationale = _string(entry["rationale"], f"waivers[{index}].rationale")
        reference = _string(entry["reference"], f"waivers[{index}].reference")
        expected_id = _stable_id(
            package,
            source_path,
            function,
            genre,
            replacement,
            source_hash,
            ordinal,
        )
        if stable_id != expected_id:
            raise MutationEvidenceError(f"waivers[{index}] identity fingerprint is invalid")
        if stable_id in waivers:
            raise MutationEvidenceError(f"duplicate mutant waiver {stable_id}")
        waivers[stable_id] = MutationWaiver(
            stable_id=stable_id,
            package=package,
            path=source_path,
            function=function,
            genre=genre,
            replacement=replacement,
            source_slice_sha256=source_hash,
            ordinal=ordinal,
            rationale=rationale,
            reference=reference,
        )
    return waivers


def apply_waivers(
    run: MutationRun,
    waivers: dict[str, MutationWaiver],
    *,
    require_all: bool,
) -> MutationRun:
    matched: set[str] = set()
    mutants: list[MutationRecord] = []
    for mutant in run.mutants:
        waiver = waivers.get(mutant.stable_id)
        if waiver is None:
            mutants.append(mutant)
            continue
        identity = (
            mutant.package,
            mutant.path,
            mutant.function,
            mutant.genre,
            mutant.replacement,
            mutant.source_slice_sha256,
            mutant.ordinal,
        )
        waiver_identity = (
            waiver.package,
            waiver.path,
            waiver.function,
            waiver.genre,
            waiver.replacement,
            waiver.source_slice_sha256,
            waiver.ordinal,
        )
        if identity != waiver_identity:
            raise MutationEvidenceError(f"mutant waiver identity mismatch: {waiver.stable_id}")
        if mutant.outcome != "missed":
            if require_all:
                raise MutationEvidenceError(
                    f"mutant waiver no longer describes a miss: {waiver.stable_id}"
                )
            mutants.append(mutant)
            continue
        matched.add(waiver.stable_id)
        mutants.append(
            replace(
                mutant,
                outcome="waived",
                metadata={
                    "waiver": {
                        "rationale": waiver.rationale,
                        "reference": waiver.reference,
                    }
                },
            )
        )
    if require_all:
        stale = sorted(set(waivers) - matched)
        if stale:
            raise MutationEvidenceError(
                "stale mutant waivers: " + ", ".join(stale)
            )
    return replace(run, mutants=mutants)


def _normalize_mutant(
    outcome: dict[str, Any],
    output_dir: Path,
    source_root: Path,
) -> MutationRecord:
    scenario = _object(outcome.get("scenario"), "outcome.scenario")
    if set(scenario) != {"Mutant"}:
        raise MutationEvidenceError("mutant scenario must contain only 'Mutant'")
    mutant = _object(scenario["Mutant"], "outcome.scenario.Mutant")
    if set(mutant) != {
        "name",
        "package",
        "file",
        "function",
        "span",
        "replacement",
        "genre",
    }:
        raise MutationEvidenceError("mutant has unexpected fields")
    package = _string(mutant.get("package"), "mutant.package")
    source_relative = _normalized_relative_path(mutant.get("file"), "mutant.file")
    source_path = (source_root / source_relative).resolve()
    try:
        source_path.relative_to(source_root.resolve())
    except ValueError as error:
        raise MutationEvidenceError("mutant.file escapes the source root") from error
    if not source_path.is_file():
        raise MutationEvidenceError(f"mutant source file is missing: {source_relative}")
    source_slice, line, column = _source_slice(source_path, mutant.get("span"))
    source_slice_sha256 = hashlib.sha256(source_slice).hexdigest()
    function = _function_name(mutant.get("function"))
    genre = _string(mutant.get("genre"), "mutant.genre")
    replacement = _string(mutant.get("replacement"), "mutant.replacement", empty=True)
    summary = _string(outcome.get("summary"), "outcome.summary")
    _phase_results(outcome.get("phase_results"), summary, "outcome")
    try:
        normalized_outcome = OUTCOME_NAMES[summary]
    except KeyError as error:
        raise MutationEvidenceError(f"unsupported mutant outcome {summary!r}") from error
    log_relative, _ = _evidence_path(output_dir, outcome.get("log_path"), "outcome.log_path")
    diff_value = outcome.get("diff_path")
    if diff_value is None:
        raise MutationEvidenceError("mutant outcome has no diff_path")
    diff_relative, _ = _evidence_path(output_dir, diff_value, "outcome.diff_path")
    return MutationRecord(
        stable_id="",
        package=package,
        path=source_relative,
        function=function,
        genre=genre,
        replacement=replacement,
        source_slice_sha256=source_slice_sha256,
        ordinal=-1,
        source_slice=source_slice,
        outcome=normalized_outcome,
        name=_string(mutant.get("name"), "mutant.name"),
        line=line,
        column=column,
        log_path=log_relative,
        diff_path=diff_relative,
        metadata={},
    )


def _assign_identities(mutants: list[MutationRecord]) -> list[MutationRecord]:
    groups: dict[tuple[Any, ...], list[int]] = {}
    for index, mutant in enumerate(mutants):
        key = (
            mutant.package,
            mutant.path,
            mutant.function,
            mutant.genre,
            mutant.replacement,
            mutant.source_slice_sha256,
        )
        groups.setdefault(key, []).append(index)
    assigned = list(mutants)
    for indices in groups.values():
        ordered = sorted(indices, key=lambda i: (mutants[i].line, mutants[i].column))
        for ordinal, index in enumerate(ordered):
            mutant = mutants[index]
            assigned[index] = replace(
                mutant,
                ordinal=ordinal,
                stable_id=_stable_id(
                    mutant.package,
                    mutant.path,
                    mutant.function,
                    mutant.genre,
                    mutant.replacement,
                    mutant.source_slice_sha256,
                    ordinal,
                ),
            )
    return assigned


def normalize_outcomes(output_dir: Path, source_root: Path) -> MutationRun:
    try:
        document = json.loads((output_dir / "outcomes.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise MutationEvidenceError(f"could not read outcomes.json: {error}") from error
    document = _object(document, "outcomes.json")
    if set(document) != {
        "outcomes",
        "total_mutants",
        "missed",
        "caught",
        "timeout",
        "unviable",
        "success",
        "start_time",
        "end_time",
        "cargo_mutants_version",
    }:
        raise MutationEvidenceError("outcomes.json has unexpected fields")
    version = _string(document.get("cargo_mutants_version"), "cargo_mutants_version")
    if version != CARGO_MUTANTS_VERSION:
        raise MutationEvidenceError(
            f"cargo-mutants version {version!r}, expected {CARGO_MUTANTS_VERSION!r}"
        )
    start_time, start_timestamp = _timestamp(document.get("start_time"), "start_time")
    end_time, end_timestamp = _timestamp(document.get("end_time"), "end_time")
    if end_timestamp < start_timestamp:
        raise MutationEvidenceError("end_time precedes start_time")
    outcomes = document.get("outcomes")
    if not isinstance(outcomes, list):
        raise MutationEvidenceError("outcomes must be an array")

    baselines = [outcome for outcome in outcomes if _object(outcome, "outcome").get("scenario") == "Baseline"]
    if len(baselines) != 1:
        raise MutationEvidenceError("outcomes must contain exactly one baseline scenario")
    baseline = _object(baselines[0], "baseline outcome")
    for index, outcome in enumerate(outcomes):
        if set(_object(outcome, f"outcomes[{index}]")) != {
            "scenario",
            "summary",
            "log_path",
            "diff_path",
            "phase_results",
        }:
            raise MutationEvidenceError(f"outcomes[{index}] has unexpected fields")
    baseline_passed = baseline.get("summary") == "Success"
    if not baseline_passed:
        raise MutationEvidenceError(
            f"unmodified baseline did not pass: {baseline.get('summary')!r}"
        )
    _evidence_path(output_dir, baseline.get("log_path"), "baseline.log_path")
    if baseline.get("diff_path") is not None:
        raise MutationEvidenceError("baseline.diff_path must be null")
    _phase_results(
        baseline.get("phase_results"),
        "Success",
        "baseline",
        baseline=True,
    )

    mutant_outcomes = [outcome for outcome in outcomes if outcome is not baselines[0]]
    mutants = [
        _normalize_mutant(_object(outcome, "mutant outcome"), output_dir, source_root)
        for outcome in mutant_outcomes
    ]
    mutants = _assign_identities(mutants)
    total = _nonnegative_integer(document.get("total_mutants"), "total_mutants")
    if total != len(mutants):
        raise MutationEvidenceError(
            f"total_mutants={total!r} does not match {len(mutants)} outcomes"
        )
    observed = {name: 0 for name in ("caught", "missed", "timeout", "unviable")}
    for mutant in mutants:
        observed[mutant.outcome] += 1
    for name, count in observed.items():
        reported = _nonnegative_integer(document.get(name), name)
        if reported != count:
            raise MutationEvidenceError(
                f"{name}={reported!r} does not match observed {count}"
            )
    if _nonnegative_integer(document.get("success"), "success") != 0:
        raise MutationEvidenceError("mutant success count must be zero for a test run")
    ids = [mutant.stable_id for mutant in mutants]
    if len(ids) != len(set(ids)):
        raise MutationEvidenceError("normalized mutant identities are not unique")
    return MutationRun(version, baseline_passed, start_time, end_time, mutants)


def diff_decision(
    scoped_changed_paths: list[str],
    scoped_untracked_rust: list[str],
    candidate_count: int,
    production_changed: bool = True,
) -> str:
    if scoped_untracked_rust:
        paths = ", ".join(sorted(scoped_untracked_rust))
        raise MutationEvidenceError(f"untracked Rust in mutation scope: {paths}")
    if not scoped_changed_paths:
        return "skip"
    if candidate_count == 0:
        if not production_changed:
            return "skip"
        raise MutationEvidenceError("scoped production lines changed but generated zero mutants")
    return "run"


def _run_git(*arguments: str, check: bool = True) -> str:
    result = subprocess.run(
        ["git", *arguments],
        cwd=REPOSITORY,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if check and result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise MutationEvidenceError(f"git {' '.join(arguments)} failed: {detail}")
    return result.stdout


def _config_hash() -> str:
    try:
        return hashlib.sha256(CONFIG.read_bytes()).hexdigest()
    except OSError as error:
        raise MutationEvidenceError(f"could not read {CONFIG}: {error}") from error


def _environment() -> dict[str, Any]:
    commit = _run_git("rev-parse", "HEAD").strip()
    dirty = bool(_run_git("status", "--porcelain", "--untracked-files=all").strip())
    return {
        "git_commit": commit,
        "git_dirty": dirty,
        "os": platform.system().lower(),
        "arch": platform.machine(),
        "cargo_profile": "test",
        "tools": {"cargo-mutants": CARGO_MUTANTS_VERSION},
        "metadata": {"mutation_config_sha256": _config_hash()},
    }


def _scope() -> list[str]:
    document = load_toml(CONFIG)
    expected = {
        "additional_cargo_args": ["--locked"],
        "test_workspace": False,
        "timeout_multiplier": 3.0,
        "minimum_test_timeout": 20.0,
        "examine_globs": EXPECTED_SCOPE,
    }
    if document != expected:
        raise MutationEvidenceError("mutation config differs from qualification-v1")
    return EXPECTED_SCOPE


def _missing_viable_scope(
    mutants: list[MutationRecord], scope: list[str]
) -> list[str]:
    return [
        pattern
        for pattern in scope
        if not any(
            mutant.outcome != "unviable"
            and fnmatch.fnmatchcase(mutant.path, pattern)
            for mutant in mutants
        )
    ]


def _check_tool() -> None:
    executable = shutil.which("cargo-mutants")
    if executable is None:
        raise MutationEvidenceError("cargo-mutants is not available")
    result = subprocess.run(
        [executable, "mutants", "--version"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    expected = f"cargo-mutants {CARGO_MUTANTS_VERSION}"
    if result.returncode != 0 or result.stdout.strip() != expected:
        raise MutationEvidenceError(
            f"cargo-mutants version mismatch: {result.stdout.strip()!r}"
        )


def _new_run(mode: str) -> tuple[Path, dict[str, Any]]:
    document = new_document("mutation-run", mode, _environment())
    run_dir = OUTPUT_ROOT / document["run_id"]
    run_dir.mkdir(parents=True, exist_ok=False)
    return run_dir, document


def _write_run(run_dir: Path, document: dict[str, Any]) -> Path:
    document["generated_at"] = utc_now()
    report = run_dir / "qualification-v1.json"
    write_qualification(report, document, SCHEMA)
    LATEST.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(report, LATEST)
    return report


def _complete_report(
    document: dict[str, Any],
    run: MutationRun,
    output_dir: Path,
    driver_log: Path,
    elapsed: float,
    native_exit: int,
    raw_counts: dict[str, int],
) -> tuple[int, int]:
    counts = {
        name: 0 for name in ("caught", "missed", "timeout", "unviable", "waived")
    }
    for mutant in run.mutants:
        counts[mutant.outcome] += 1
    if not run.mutants:
        raise MutationEvidenceError("cargo-mutants produced no mutants")
    if document["mode"] == "mutants-full":
        missing_viable = _missing_viable_scope(run.mutants, _scope())
        if missing_viable:
            raise MutationEvidenceError(
                "mutation scope entries produced no viable mutant: "
                + ", ".join(missing_viable)
            )

    baseline_status, baseline_outcomes = _load_baseline()
    current = {mutant.stable_id: mutant.outcome for mutant in run.mutants}
    regressions, new_ids = compare_mutant_outcomes(baseline_outcomes, current)
    expected_native_exit = (
        3 if raw_counts["timeout"] else 2 if raw_counts["missed"] else 0
    )
    if native_exit != expected_native_exit:
        raise MutationEvidenceError(
            f"cargo-mutants exit {native_exit} conflicts with normalized outcome "
            f"(expected {expected_native_exit})"
        )
    if elapsed > WALL_CLOCK_SECONDS:
        raise MutationEvidenceError("cargo-mutants wall-clock limit exceeded")

    document["mutants"] = [mutant.as_report_record() for mutant in run.mutants]
    document["summary"] = {
        "total": len(run.mutants),
        "passed": counts["caught"] + counts["waived"],
        "failed": counts["missed"],
        "invalid": counts["timeout"],
        "skipped": counts["unviable"],
        "counts": {**counts, "regression": len(regressions), "new": len(new_ids)},
    }
    document["metadata"].update(
        {
            "baseline": baseline_status,
            "baseline_regressions": regressions,
            "new_identities": new_ids,
            "cargo_mutants_native_exit": native_exit,
            "elapsed_seconds": elapsed,
            "wall_clock_limit_seconds": WALL_CLOCK_SECONDS,
            "cargo_mutants_started_at": run.start_time,
            "cargo_mutants_ended_at": run.end_time,
        }
    )
    if baseline_status == "absent":
        print(f"BASELINE mutants-{document['mode'].removeprefix('mutants-')}: absent")
    document["artifacts"] = [
        artifact_record("cargo-mutants-output", output_dir, REPOSITORY),
        artifact_record("cargo-mutants-driver-log", driver_log, REPOSITORY),
    ]
    for kind, path in (
        ("mutants-diff", output_dir.parent / "changes.patch"),
        ("mutants-candidates", output_dir.parent / "candidates.json"),
    ):
        if path.exists():
            document["artifacts"].append(artifact_record(kind, path, REPOSITORY))
    if counts["timeout"]:
        document["outcome"] = "error"
        document["errors"] = [
            error_record(
                "timeout",
                "cargo-mutants reported timed-out mutants",
                document["mode"],
            )
        ]
        return 2, len(regressions)
    document["complete"] = True
    document["outcome"] = "fail" if counts["missed"] else "pass"
    return qualification_exit(document), len(regressions)


def _load_baseline() -> tuple[str, dict[str, str]]:
    if not BASELINE.exists():
        return "absent", {}
    baseline = load_qualification(BASELINE, SCHEMA, require_complete=True)
    if baseline.get("evidence_kind") != "mutation-baseline":
        raise MutationEvidenceError("committed mutation baseline has the wrong evidence kind")
    if baseline.get("outcome") != "pass":
        raise MutationEvidenceError("committed mutation baseline is not passing evidence")
    tools = baseline["environment"]["tools"]
    metadata = baseline["environment"]["metadata"]
    if tools.get("cargo-mutants") != CARGO_MUTANTS_VERSION:
        raise MutationEvidenceError("committed baseline cargo-mutants version is stale")
    if metadata.get("mutation_config_sha256") != _config_hash():
        raise MutationEvidenceError("committed baseline mutation config hash is stale")
    outcomes = {record["id"]: record["outcome"] for record in baseline["mutants"]}
    if any(outcome in {"missed", "timeout"} for outcome in outcomes.values()):
        raise MutationEvidenceError("committed mutation baseline contains missed or timed-out mutants")
    return "present", outcomes


def _record_error(run_dir: Path, document: dict[str, Any], error: BaseException) -> int:
    document["complete"] = False
    document["outcome"] = "error"
    document["errors"] = [error_record("invalid-evidence", str(error), document["mode"])]
    document["summary"]["invalid"] = 1
    artifacts = []
    for kind, path in (
        ("cargo-mutants-output", run_dir / "mutants.out"),
        ("cargo-mutants-driver-log", run_dir / "cargo-mutants.log"),
        ("mutants-diff", run_dir / "changes.patch"),
        ("mutants-candidates", run_dir / "candidates.json"),
    ):
        if path.exists():
            artifacts.append(artifact_record(kind, path, REPOSITORY))
    document["artifacts"] = artifacts
    _write_run(run_dir, document)
    return 2


def _kill_process_group(process: subprocess.Popen[Any]) -> None:
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    except OSError:
        process.kill()
    process.wait()


def _cargo_mutants(
    output_dir: Path,
    driver_log: Path,
    extra_arguments: list[str],
    started: float,
) -> tuple[int, float]:
    command = [
        "cargo",
        "mutants",
        "--jobs",
        os.environ.get("QUALIFICATION_JOBS", "3"),
        "--no-shuffle",
        "--output",
        str(output_dir),
        *extra_arguments,
    ]
    remaining = WALL_CLOCK_SECONDS - (time.monotonic() - started)
    if remaining <= 0:
        raise MutationEvidenceError("cargo-mutants wall-clock limit exceeded")
    with driver_log.open("w", encoding="utf-8") as handle:
        handle.write("command: " + " ".join(command) + "\n")
        handle.flush()
        process: subprocess.Popen[Any] | None = None
        try:
            process = subprocess.Popen(
                command,
                cwd=REPOSITORY,
                stdout=handle,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            return_code = process.wait(timeout=remaining)
        except subprocess.TimeoutExpired as error:
            if process is not None:
                _kill_process_group(process)
            raise MutationEvidenceError(
                "cargo-mutants wall-clock limit exceeded"
            ) from error
        except BaseException:
            if process is not None:
                _kill_process_group(process)
            raise
    return return_code, time.monotonic() - started


def _print_survivors(run: MutationRun, output_dir: Path) -> None:
    for mutant in run.mutants:
        if mutant.outcome != "missed":
            continue
        print(f"SURVIVOR {mutant.stable_id}")
        print(f"  location: {mutant.path}:{mutant.line}:{mutant.column}")
        print(f"  mutation: {mutant.name}")
        if mutant.log_path is not None:
            log = output_dir / mutant.log_path
            print(f"  raw log: {log}")
            print(log.read_text(encoding="utf-8", errors="replace"), end="")
            if log.stat().st_size and not log.read_bytes().endswith(b"\n"):
                print()
        escaped = re.sub(r"([\\.^$|?*+()\[\]{}])", r"\\\1", mutant.name)
        print(
            "  rerun: cargo mutants --jobs 1 --no-shuffle "
            f"--re '^{escaped}$'"
        )


def _execute_mutants(
    run_dir: Path,
    document: dict[str, Any],
    extra_arguments: list[str],
    terminal_mode: str,
    *,
    started: float | None = None,
) -> int:
    output_dir = run_dir / "mutants.out"
    driver_log = run_dir / "cargo-mutants.log"
    if started is None:
        started = time.monotonic()
    # cargo-mutants creates a mutants.out/ directory inside --output
    native_exit, elapsed = _cargo_mutants(
        run_dir, driver_log, extra_arguments, started
    )
    run = normalize_outcomes(output_dir, REPOSITORY)
    raw_counts = {name: 0 for name in ("caught", "missed", "timeout", "unviable")}
    for mutant in run.mutants:
        raw_counts[mutant.outcome] += 1
    run = apply_waivers(
        run,
        load_waivers(),
        require_all=terminal_mode == "full",
    )
    exit_code, regressions = _complete_report(
        document,
        run,
        output_dir,
        driver_log,
        elapsed,
        native_exit,
        raw_counts,
    )
    _write_run(run_dir, document)
    _print_survivors(run, output_dir)
    if exit_code == 0:
        if terminal_mode == "full":
            print(
                "PASS mutants-full: missed=0 timeout=0 "
                f"regression={regressions} elapsed<=1800s"
            )
        else:
            print("PASS mutants-diff: complete, no missed mutants")
    elif exit_code == 1:
        print(f"FAIL mutants-{terminal_mode}: missed mutants")
    else:
        print(f"ERROR mutants-{terminal_mode}: incomplete evidence")
    return exit_code


def run_full() -> int:
    try:
        run_dir, document = _new_run("mutants-full")
        _check_tool()
        _scope()
        return _execute_mutants(run_dir, document, [], "full")
    except BaseException as error:
        if "run_dir" in locals():
            try:
                _record_error(run_dir, document, error)
            except BaseException as report_error:
                print(f"ERROR mutants-full report: {report_error}", file=sys.stderr)
        print("ERROR mutants-full: incomplete evidence")
        return 2


def _matches_scope(path: str, scope: list[str]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in scope)


def _test_module_range(source: str) -> tuple[int, int] | None:
    lines = source.splitlines()
    marker = re.compile(r"^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*$")
    module = re.compile(r"^\s*mod\s+tests\s*\{")
    for index, line in enumerate(lines):
        if not marker.match(line):
            continue
        next_index = index + 1
        while next_index < len(lines) and not lines[next_index].strip():
            next_index += 1
        if next_index < len(lines) and module.match(lines[next_index]):
            for closing_index in range(next_index + 1, len(lines)):
                if lines[closing_index] == "}":
                    return index + 1, closing_index + 1
    return None


def _changed_line_is_production(
    line_number: int,
    content: str,
    test_module_range: tuple[int, int] | None,
) -> bool:
    if (
        test_module_range is not None
        and test_module_range[0] <= line_number <= test_module_range[1]
    ):
        return False
    stripped = content.strip()
    return bool(stripped) and not stripped.startswith(("//", "/*", "*", "*/"))


def _diff_has_production_changes(
    diff: str, old_source: str, new_source: str
) -> bool:
    old_test_range = _test_module_range(old_source)
    new_test_range = _test_module_range(new_source)
    hunk = re.compile(
        r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@"
    )
    old_line: int | None = None
    new_line: int | None = None
    for raw in diff.splitlines():
        match = hunk.match(raw)
        if match:
            old_line = int(match.group(1))
            new_line = int(match.group(3))
            continue
        if old_line is None or new_line is None:
            continue
        if raw.startswith("-"):
            if _changed_line_is_production(old_line, raw[1:], old_test_range):
                return True
            old_line += 1
        elif raw.startswith("+"):
            if _changed_line_is_production(new_line, raw[1:], new_test_range):
                return True
            new_line += 1
        elif raw.startswith(" "):
            old_line += 1
            new_line += 1
        elif not raw.startswith("\\ No newline at end of file"):
            raise MutationEvidenceError("could not parse scoped git diff")
    return False


def _git_source(revision: str, path: str) -> str:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=REPOSITORY,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        stderr = result.stderr.decode("utf-8", errors="replace")
        if "does not exist" in stderr or "exists on disk, but not in" in stderr:
            return ""
        detail = stderr.strip()
        raise MutationEvidenceError(f"git show {revision}:{path} failed: {detail}")
    try:
        return result.stdout.decode("utf-8")
    except UnicodeDecodeError as error:
        raise MutationEvidenceError(f"non-UTF-8 Rust source at {revision}:{path}") from error


def _scoped_production_changed(merge_base: str, paths: list[str]) -> bool:
    for path in paths:
        diff = _run_git(
            "diff",
            "--no-ext-diff",
            "--unified=0",
            merge_base,
            "--",
            path,
        )
        old_source = _git_source(merge_base, path)
        current = REPOSITORY / path
        try:
            new_source = current.read_bytes().decode("utf-8") if current.exists() else ""
        except (OSError, UnicodeDecodeError) as error:
            raise MutationEvidenceError(f"could not read changed Rust source {path}") from error
        if _diff_has_production_changes(diff, old_source, new_source):
            return True
    return False


def _candidate_count(document: Any) -> int:
    if not isinstance(document, list) or not all(isinstance(item, dict) for item in document):
        raise MutationEvidenceError("cargo-mutants candidate JSON has an invalid shape")
    if not all(isinstance(item.get("name"), str) and item["name"] for item in document):
        raise MutationEvidenceError("cargo-mutants candidate JSON has a nameless mutant")
    return len(document)


def _list_candidates(diff_path: Path, destination: Path, started: float) -> int:
    remaining = WALL_CLOCK_SECONDS - (time.monotonic() - started)
    if remaining <= 0:
        raise MutationEvidenceError("cargo-mutants wall-clock limit exceeded")
    process: subprocess.Popen[str] | None = None
    try:
        process = subprocess.Popen(
            ["cargo", "mutants", "--list", "--json", "--in-diff", str(diff_path)],
            cwd=REPOSITORY,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
        stdout, stderr = process.communicate(timeout=remaining)
    except subprocess.TimeoutExpired as error:
        if process is not None:
            _kill_process_group(process)
            process.communicate()
        raise MutationEvidenceError("cargo-mutants wall-clock limit exceeded") from error
    except BaseException:
        if process is not None:
            _kill_process_group(process)
            process.communicate()
        raise
    if process.returncode != 0:
        detail = stderr.strip() or stdout.strip()
        raise MutationEvidenceError(f"could not list diff mutants: {detail}")
    try:
        candidates = json.loads(stdout)
    except json.JSONDecodeError as error:
        raise MutationEvidenceError(f"invalid candidate JSON: {error}") from error
    count = _candidate_count(candidates)
    destination.write_text(json.dumps(candidates, indent=2) + "\n", encoding="utf-8")
    print(f"CANDIDATES mutants-diff: {count}")
    for candidate in candidates:
        print(f"CANDIDATE {candidate['name']}")
    return count


def run_diff(base: str) -> int:
    try:
        run_dir, document = _new_run("mutants-diff")
        _check_tool()
        merge_base = _run_git("merge-base", base, "HEAD").strip()
        if not merge_base:
            raise MutationEvidenceError(f"no merge base for {base!r}")
        diff_path = run_dir / "changes.patch"
        diff_path.write_text(_run_git("diff", "--binary", merge_base), encoding="utf-8")
        changed = [
            path
            for path in _run_git("diff", "--name-only", merge_base).splitlines()
            if path
        ]
        scope = _scope()
        scoped_changed = sorted(path for path in changed if _matches_scope(path, scope))
        untracked = [
            path
            for path in _run_git(
                "ls-files", "--others", "--exclude-standard", "--", "*.rs"
            ).splitlines()
            if path and _matches_scope(path, scope)
        ]
        if untracked:
            diff_decision(scoped_changed, untracked, 0)
        if not scoped_changed:
            document["complete"] = True
            document["outcome"] = "skip"
            document["summary"]["skipped"] = 1
            document["metadata"] = {
                "base": base,
                "merge_base": merge_base,
                "scoped_changed_paths": [],
            }
            document["artifacts"] = [
                artifact_record("mutants-diff", diff_path, REPOSITORY)
            ]
            _write_run(run_dir, document)
            print("SKIP mutants-diff: no mutable production lines changed")
            return 0
        started = time.monotonic()
        candidates_path = run_dir / "candidates.json"
        candidate_count = _list_candidates(diff_path, candidates_path, started)
        production_changed = _scoped_production_changed(merge_base, scoped_changed)
        decision = diff_decision(
            scoped_changed,
            untracked,
            candidate_count,
            production_changed,
        )
        document["metadata"].update(
            {
                "base": base,
                "merge_base": merge_base,
                "scoped_changed_paths": scoped_changed,
                "candidate_count": candidate_count,
                "production_changed": production_changed,
            }
        )
        if decision == "skip":
            document["complete"] = True
            document["outcome"] = "skip"
            document["summary"]["skipped"] = 1
            document["artifacts"] = [
                artifact_record("mutants-diff", diff_path, REPOSITORY),
                artifact_record("mutants-candidates", candidates_path, REPOSITORY),
            ]
            _write_run(run_dir, document)
            print("SKIP mutants-diff: no mutable production lines changed")
            return 0
        return _execute_mutants(
            run_dir,
            document,
            ["--in-diff", str(diff_path)],
            "diff",
            started=started,
        )
    except BaseException as error:
        if "run_dir" in locals():
            try:
                _record_error(run_dir, document, error)
            except BaseException as report_error:
                print(f"ERROR mutants-diff report: {report_error}", file=sys.stderr)
        print("ERROR mutants-diff: incomplete evidence")
        return 2


def write_baseline_candidate(source: Path, destination: Path) -> int:
    try:
        committed_destination = BASELINE.resolve()
        if destination.resolve() == committed_destination:
            raise MutationEvidenceError("baseline candidate cannot overwrite the committed baseline")
        report = load_qualification(source, SCHEMA, require_complete=True)
        if report.get("evidence_kind") != "mutation-run" or report.get("mode") != "mutants-full":
            raise MutationEvidenceError("baseline candidate requires a complete full mutation run")
        if report.get("outcome") != "pass":
            raise MutationEvidenceError("baseline candidate requires a passing mutation run")
        counts = report["summary"]["counts"]
        if counts.get("missed") != 0 or counts.get("timeout") != 0:
            raise MutationEvidenceError("baseline candidate contains missed or timed-out mutants")
        head = _run_git("rev-parse", "HEAD").strip()
        if report["environment"]["git_commit"] != head:
            raise MutationEvidenceError("mutation run is not from current HEAD")
        if report["environment"]["git_dirty"]:
            raise MutationEvidenceError("mutation run recorded a dirty worktree")
        if report["environment"]["tools"].get("cargo-mutants") != CARGO_MUTANTS_VERSION:
            raise MutationEvidenceError("mutation run has the wrong cargo-mutants version")
        if (
            report["environment"]["metadata"].get("mutation_config_sha256")
            != _config_hash()
        ):
            raise MutationEvidenceError("mutation run has a stale config hash")
        if _run_git("status", "--porcelain", "--untracked-files=all").strip():
            raise MutationEvidenceError("current worktree is dirty")
        candidate = dict(report)
        candidate["evidence_kind"] = "mutation-baseline"
        candidate["mode"] = "mutants-baseline"
        candidate["run_id"] = report["run_id"] + "-baseline-candidate"
        candidate["generated_at"] = utc_now()
        candidate["artifacts"] = []
        candidate["mutants"] = [
            {**record, "log_path": None, "diff_path": None}
            for record in report["mutants"]
        ]
        candidate["metadata"] = {
            **report["metadata"],
            "candidate_from": str(source),
        }
        write_qualification(destination, candidate, SCHEMA)
    except BaseException as error:
        print(f"ERROR mutants-baseline candidate: {error}", file=sys.stderr)
        return 2
    print("PASS mutants-baseline candidate: complete current-head clean-tree")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("full")
    diff_parser = subparsers.add_parser("diff")
    diff_parser.add_argument("--base", required=True)
    baseline_parser = subparsers.add_parser("baseline-candidate")
    baseline_parser.add_argument("--from", dest="source", type=Path, required=True)
    baseline_parser.add_argument(
        "--output",
        type=Path,
        default=REPOSITORY / "target" / "qualification" / "mutants-baseline-candidate-v1.json",
    )
    arguments = parser.parse_args()
    if arguments.command == "full":
        return run_full()
    if arguments.command == "diff":
        return run_diff(arguments.base)
    return write_baseline_candidate(arguments.source, arguments.output)


if __name__ == "__main__":
    raise SystemExit(main())
