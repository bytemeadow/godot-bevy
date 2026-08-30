#!/usr/bin/env python3
from __future__ import annotations

import argparse
import platform
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from qualification_faults import run_fault_pack
from qualification_schema import (
    QualificationValidationError,
    error_record,
    load_qualification,
    new_document,
    utc_now,
    write_qualification,
)
from qualification_toml import TomlError, load_toml

REPOSITORY = Path(__file__).resolve().parents[1]
REGISTRY = REPOSITORY / "itest" / "qualification" / "checks-v1.toml"
SCHEMA = REPOSITORY / "itest" / "schema" / "qualification-v1.schema.json"
OUTPUT_ROOT = REPOSITORY / "target" / "qualification" / "aggregate"


class AggregateError(ValueError):
    pass


@dataclass(frozen=True)
class RegisteredCheck:
    id: str
    command: tuple[str, ...]
    artifact: Path
    evidence_kind: str


def _git(*arguments: str) -> str:
    result = subprocess.run(
        ["git", *arguments],
        cwd=REPOSITORY,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise AggregateError(f"git {' '.join(arguments)} failed: {detail}")
    return result.stdout


def _environment() -> dict[str, Any]:
    return {
        "git_commit": _git("rev-parse", "HEAD").strip(),
        "git_dirty": bool(
            _git("status", "--porcelain", "--untracked-files=all").strip()
        ),
        "os": platform.system().lower(),
        "arch": platform.machine(),
        "cargo_profile": None,
        "tools": {},
        "metadata": {},
    }


def load_registry(path: Path = REGISTRY) -> list[RegisteredCheck]:
    try:
        document = load_toml(path)
    except TomlError as error:
        raise AggregateError(str(error)) from error
    if set(document) != {"version", "checks"}:
        raise AggregateError("qualification check registry has invalid top-level fields")
    if document.get("version") != 1:
        raise AggregateError("qualification check registry version must be 1")
    raw_checks = document.get("checks")
    if not isinstance(raw_checks, list) or not raw_checks:
        raise AggregateError("qualification check registry is empty")
    checks: list[RegisteredCheck] = []
    for index, raw in enumerate(raw_checks):
        if not isinstance(raw, dict) or set(raw) != {
            "id",
            "command",
            "artifact",
            "evidence_kind",
        }:
            raise AggregateError(f"checks[{index}] has invalid fields")
        identifier = raw["id"]
        command = raw["command"]
        artifact = raw["artifact"]
        evidence_kind = raw["evidence_kind"]
        if not isinstance(identifier, str) or not identifier:
            raise AggregateError(f"checks[{index}].id must be a nonempty string")
        if not isinstance(command, list) or not command or not all(
            isinstance(argument, str) and argument for argument in command
        ):
            raise AggregateError(f"checks[{index}].command must be a string array")
        if not isinstance(artifact, str) or not artifact:
            raise AggregateError(f"checks[{index}].artifact must be a nonempty string")
        if not isinstance(evidence_kind, str) or not evidence_kind:
            raise AggregateError(
                f"checks[{index}].evidence_kind must be a nonempty string"
            )
        artifact_path = Path(artifact)
        if artifact_path.is_absolute() or ".." in artifact_path.parts:
            raise AggregateError(f"checks[{index}].artifact escapes the repository")
        artifact_path = REPOSITORY / artifact_path
        artifact_root = REPOSITORY / "target" / "qualification"
        try:
            artifact_path.resolve().relative_to(artifact_root.resolve())
        except ValueError as error:
            raise AggregateError(
                f"checks[{index}].artifact must be under target/qualification"
            ) from error
        checks.append(
            RegisteredCheck(
                identifier,
                tuple(command),
                artifact_path,
                evidence_kind,
            )
        )
    ids = [check.id for check in checks]
    if len(ids) != len(set(ids)):
        raise AggregateError("qualification check registry has duplicate ids")
    return checks


def _run_full() -> int:
    try:
        checks = load_registry()
        document = new_document("aggregate", "full", _environment())
    except BaseException as error:
        print(f"ERROR qualification: {error}")
        return 2
    run_dir = OUTPUT_ROOT / document["run_id"]
    run_dir.mkdir(parents=True, exist_ok=False)
    worst_exit = 0
    errors: list[dict[str, Any]] = []
    for check in checks:
        try:
            check.artifact.unlink(missing_ok=True)
            result = subprocess.run(check.command, cwd=REPOSITORY, check=False)
            exit_code = result.returncode if result.returncode in {0, 1, 2} else 2
            evidence = load_qualification(check.artifact, SCHEMA)
            if evidence["evidence_kind"] != check.evidence_kind:
                raise AggregateError(
                    f"child evidence kind {evidence['evidence_kind']!r}, "
                    f"expected {check.evidence_kind!r}"
                )
            if exit_code in {0, 1} and not evidence["complete"]:
                raise AggregateError("child returned complete exit with incomplete evidence")
            expected_exit = {
                "pass": 0,
                "skip": 0,
                "fail": 1,
                "error": 2,
                "incomplete": 2,
            }[evidence["outcome"]]
            if exit_code != expected_exit:
                raise AggregateError(
                    f"child exit {exit_code} conflicts with evidence {evidence['outcome']}"
                )
            outcome = {0: "pass", 1: "fail", 2: "error"}[exit_code]
            if evidence["outcome"] == "skip":
                outcome = "skip"
            artifact = check.artifact.resolve().relative_to(REPOSITORY.resolve()).as_posix()
        except (
            AggregateError,
            QualificationValidationError,
            OSError,
            KeyError,
            ValueError,
        ) as error:
            exit_code = 2
            outcome = "error"
            artifact = None
            errors.append(error_record("invalid-evidence", str(error), check.id))
        document["checks"].append(
            {
                "id": check.id,
                "outcome": outcome,
                "exit_code": exit_code,
                "artifact": artifact,
                "metadata": {},
            }
        )
        worst_exit = max(worst_exit, exit_code)
    if any(check["outcome"] == "skip" for check in document["checks"]):
        worst_exit = max(worst_exit, 1)

    document["summary"] = {
        "total": len(checks),
        "passed": sum(check["outcome"] == "pass" for check in document["checks"]),
        "failed": sum(check["outcome"] == "fail" for check in document["checks"]),
        "invalid": sum(check["outcome"] == "error" for check in document["checks"]),
        "skipped": sum(check["outcome"] == "skip" for check in document["checks"]),
        "counts": {},
    }
    document["errors"] = errors
    if worst_exit == 2:
        document["complete"] = False
        document["outcome"] = "error"
    else:
        document["complete"] = True
        document["outcome"] = "fail" if worst_exit == 1 else "pass"
    document["generated_at"] = utc_now()
    report = run_dir / "qualification-v1.json"
    write_qualification(report, document, SCHEMA)
    latest = REPOSITORY / "target" / "qualification" / "latest-aggregate" / report.name
    latest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(report, latest)
    for check in document["checks"]:
        terminal = {
            "pass": "PASS",
            "skip": "SKIP",
            "fail": "FAIL",
            "error": "ERROR",
        }[check["outcome"]]
        print(f"{terminal} qualification: {check['id']}")
    if worst_exit == 0:
        print("QUALIFIED tier-3")
    return worst_exit


def run_full() -> int:
    try:
        return _run_full()
    except Exception as error:
        print(f"ERROR qualification: {error}")
        return 2


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    faults = subparsers.add_parser("faults")
    faults.add_argument("--profiles", required=True)
    subparsers.add_parser("full")
    arguments = parser.parse_args()
    if arguments.command == "faults":
        profiles = [
            profile.strip()
            for profile in arguments.profiles.split(",")
            if profile.strip()
        ]
        try:
            return run_fault_pack(profiles)
        except Exception as error:
            print(f"ERROR fault-pack: {error}")
            return 2
    return run_full()


if __name__ == "__main__":
    raise SystemExit(main())
