#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import json
import os
import platform
import re
import signal
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from qualification_schema import (
    QualificationValidationError,
    artifact_record,
    error_record,
    new_document,
    utc_now,
    validate_json_schema,
    write_qualification,
)
from qualification_toml import TomlError, load_toml

REPOSITORY = Path(__file__).resolve().parents[1]
FAULTS_ROOT = REPOSITORY / "itest" / "faults"
MANIFEST = FAULTS_ROOT / "manifest.toml"
QUALIFICATION_SCHEMA = (
    REPOSITORY / "itest" / "schema" / "qualification-v1.schema.json"
)
ITEST_SCHEMA = REPOSITORY / "godot-bevy-test" / "schema" / "itest-report-v1.schema.json"
OUTPUT_ROOT = REPOSITORY / "target" / "qualification" / "faults"
ITEST_TIMEOUT_SECONDS = 1800
EXPECTED_IDS = [
    "transform_bevy_write_uses_shadow",
    "transform_godot_read_is_discarded",
    "scene_remove_keeps_entity",
    "scene_reparent_is_removal",
    "signal_drain_drops_dispatch",
    "collision_end_drops_observer",
    "event_bridge_drains_twice",
    "input_edge_sticks",
    "autosync_registered_match_bypassed",
    "packed_scene_reconciliation_duplicates_entity",
    "asset_reader_returns_empty_bytes",
    "fixed_driver_ignores_pause",
]


class FaultEvidenceError(ValueError):
    pass


@dataclass(frozen=True)
class Fault:
    id: str
    patch: Path
    killer_test: str
    assertion_signature: str


@dataclass(frozen=True)
class FaultManifest:
    profiles: tuple[str, ...]
    faults: tuple[Fault, ...]


@dataclass(frozen=True)
class Attribution:
    outcome: str
    failed_tests: tuple[str, ...]
    matched_signatures: tuple[str, ...]
    error: str | None = None


def _nonempty_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise FaultEvidenceError(f"{label} must be a nonempty string")
    return value


def load_manifest(path: Path = MANIFEST) -> FaultManifest:
    try:
        document = load_toml(path)
    except TomlError as error:
        raise FaultEvidenceError(str(error)) from error
    if set(document) != {"version", "profiles", "faults"}:
        raise FaultEvidenceError("fault manifest has invalid top-level fields")
    if document.get("version") != 1:
        raise FaultEvidenceError("fault manifest version must be 1")
    profiles = document.get("profiles")
    if profiles != ["debug", "release"]:
        raise FaultEvidenceError("fault profiles must be exactly debug,release")
    raw_faults = document.get("faults")
    if not isinstance(raw_faults, list):
        raise FaultEvidenceError("faults must be an array of tables")
    faults: list[Fault] = []
    for index, raw in enumerate(raw_faults):
        if not isinstance(raw, dict) or set(raw) != {
            "id",
            "patch",
            "killer_test",
            "assertion_signature",
        }:
            raise FaultEvidenceError(f"faults[{index}] has invalid fields")
        patch_name = _nonempty_string(raw["patch"], f"faults[{index}].patch")
        if Path(patch_name).name != patch_name or not patch_name.endswith(".patch"):
            raise FaultEvidenceError(f"faults[{index}].patch must be a local patch name")
        faults.append(
            Fault(
                id=_nonempty_string(raw["id"], f"faults[{index}].id"),
                patch=path.parent / patch_name,
                killer_test=_nonempty_string(
                    raw["killer_test"], f"faults[{index}].killer_test"
                ),
                assertion_signature=_nonempty_string(
                    raw["assertion_signature"],
                    f"faults[{index}].assertion_signature",
                ),
            )
        )
    ids = [fault.id for fault in faults]
    if ids != EXPECTED_IDS:
        raise FaultEvidenceError("fault manifest does not contain the ordered twelve-fault pack")
    if len({fault.killer_test for fault in faults}) != len(faults):
        raise FaultEvidenceError("each fault must have a distinct killer test")
    patch_files = sorted(path.parent.glob("*.patch"))
    if set(patch_files) != {fault.patch for fault in faults}:
        raise FaultEvidenceError("manifest and fault patch census differ")
    test_sources = [
        source.read_text(encoding="utf-8")
        for source in sorted((REPOSITORY / "itest" / "rust" / "src").glob("*.rs"))
    ]
    registered_names = [
        name
        for source in test_sources
        for name in re.findall(r"\nfn (test_[A-Za-z0-9_]+)\(", source)
    ]
    for fault in faults:
        marker = f"fn {fault.killer_test}("
        matches = [source for source in test_sources if marker in source]
        if len(matches) != 1:
            raise FaultEvidenceError(f"missing killer test {fault.killer_test}")
        selected_names = [
            name for name in registered_names if fault.killer_test in name
        ]
        if selected_names != [fault.killer_test]:
            raise FaultEvidenceError(
                f"killer filter is not exact for {fault.killer_test}: {selected_names}"
            )
        start = matches[0].index(marker)
        end = matches[0].find("\n#[itest", start + len(marker))
        killer_source = matches[0][start : end if end >= 0 else len(matches[0])]
        if fault.assertion_signature not in killer_source:
            raise FaultEvidenceError(
                f"missing assertion signature for {fault.id}: {fault.assertion_signature!r}"
            )
    return FaultManifest(tuple(profiles), tuple(faults))


def _patch_paths(patch: Path) -> list[str]:
    try:
        lines = patch.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise FaultEvidenceError(f"could not read {patch}: {error}") from error
    paths: list[str] = []
    file_headers: list[str] = []
    for line in lines:
        if line.startswith("diff --git "):
            parts = line.split()
            if len(parts) != 4 or not parts[2].startswith("a/") or not parts[3].startswith("b/"):
                raise FaultEvidenceError(f"invalid diff header in {patch.name}")
            if parts[2][2:] != parts[3][2:]:
                raise FaultEvidenceError(f"fault patch may not rename files: {patch.name}")
            paths.append(parts[2][2:])
        elif line.startswith("--- ") or line.startswith("+++ "):
            header = line[4:]
            if not header.startswith(("a/", "b/")):
                raise FaultEvidenceError(f"invalid file header in {patch.name}")
            file_headers.append(header[2:])
    if not paths or len(paths) != len(set(paths)):
        raise FaultEvidenceError(f"fault patch has missing or duplicate paths: {patch.name}")
    for source in paths:
        if not source.startswith("godot-bevy/src/") or not source.endswith(".rs"):
            raise FaultEvidenceError(
                f"fault patch may only edit godot-bevy library source: {patch.name}: {source}"
            )
        if not (REPOSITORY / source).is_file():
            raise FaultEvidenceError(f"fault patch source is missing: {source}")
    if len(file_headers) != len(paths) * 2 or set(file_headers) != set(paths):
        raise FaultEvidenceError(f"fault patch file headers differ from diff paths: {patch.name}")
    return paths


def _apply_command(root: Path, arguments: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "apply", *arguments],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def verify_patch_applicability(manifest: FaultManifest) -> tuple[int, int]:
    applicable = 0
    reversible = 0
    for fault in manifest.faults:
        sources = _patch_paths(fault.patch)
        originals = {source: (REPOSITORY / source).read_bytes() for source in sources}
        with tempfile.TemporaryDirectory(prefix="qualification-fault-") as temporary:
            root = Path(temporary)
            for source, data in originals.items():
                destination = root / source
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(data)
            check = _apply_command(root, ["--check", str(fault.patch)])
            if check.returncode != 0:
                raise FaultEvidenceError(
                    f"{fault.id} is not applicable: {check.stderr.strip()}"
                )
            applied = _apply_command(root, [str(fault.patch)])
            if applied.returncode != 0:
                raise FaultEvidenceError(
                    f"{fault.id} apply failed: {applied.stderr.strip()}"
                )
            applicable += 1
            reverse_check = _apply_command(
                root, ["--reverse", "--check", str(fault.patch)]
            )
            if reverse_check.returncode != 0:
                raise FaultEvidenceError(
                    f"{fault.id} is not reversible: {reverse_check.stderr.strip()}"
                )
            reversed_result = _apply_command(root, ["--reverse", str(fault.patch)])
            if reversed_result.returncode != 0:
                raise FaultEvidenceError(
                    f"{fault.id} reverse failed: {reversed_result.stderr.strip()}"
                )
            if any((root / source).read_bytes() != data for source, data in originals.items()):
                raise FaultEvidenceError(f"{fault.id} reverse did not restore source bytes")
            reversible += 1
    return applicable, reversible


def _load_report(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise FaultEvidenceError(f"could not read Tier-1 report {path}: {error}") from error
    if not isinstance(document, dict):
        raise FaultEvidenceError("Tier-1 report root must be an object")
    try:
        validate_json_schema(document, ITEST_SCHEMA)
    except QualificationValidationError as error:
        raise FaultEvidenceError(f"invalid Tier-1 report: {error}") from error
    return document


def _failure_text(test: dict[str, Any]) -> str:
    values: list[str] = []
    for attempt in test["attempts"]:
        for failure in attempt["failures"]:
            for key in ("message", "location", "gdext_context", "callback"):
                value = failure.get(key)
                if isinstance(value, str):
                    values.append(value)
    return "\n".join(values)


def attribute_report(
    report: dict[str, Any],
    expected_tests: list[str],
    profile: str,
    assertion_signature: str | None,
    *,
    baseline: bool = False,
) -> Attribution:
    try:
        validate_json_schema(report, ITEST_SCHEMA)
        if profile not in {"debug", "release"}:
            raise FaultEvidenceError(f"unsupported profile {profile!r}")
        environment = report["environment"]
        if environment["build_profile"] != profile:
            raise FaultEvidenceError("Tier-1 report profile does not match the requested profile")
        if environment["debug_assertions"] != (profile == "debug"):
            raise FaultEvidenceError("Tier-1 debug_assertions/profile mismatch")
        selection = report["selection"]
        if selection["focus_run"]:
            raise FaultEvidenceError("focused Tier-1 run is invalid fault evidence")
        if selection["filter"] != ",".join(expected_tests):
            raise FaultEvidenceError("Tier-1 filter does not match manifest selection")
        if selection["patterns"] != expected_tests:
            raise FaultEvidenceError("Tier-1 patterns do not match manifest selection")
        if selection["selected"] != len(expected_tests):
            raise FaultEvidenceError("Tier-1 selected count does not match manifest selection")
        tests = report["tests"]
        names = [test["name"] for test in tests]
        if len(names) != len(set(names)) or set(names) != set(expected_tests):
            raise FaultEvidenceError("Tier-1 report tests do not equal manifest selection")
        if not report["complete"] or report["errors"]:
            raise FaultEvidenceError("Tier-1 run is incomplete or contains runner errors")
        if report["repeat"] != 1:
            raise FaultEvidenceError("fault evidence must use one attempt per killer test")
        if report["summary"]["total"] != len(tests):
            raise FaultEvidenceError("Tier-1 summary total does not match tests")
        outcome_counts = {
            outcome: sum(test["outcome"] == outcome for test in tests)
            for outcome in ("pass", "fail", "flaky", "skip")
        }
        for outcome in ("passed", "failed", "flaky", "skipped"):
            test_outcome = {
                "passed": "pass",
                "failed": "fail",
                "flaky": "flaky",
                "skipped": "skip",
            }[outcome]
            if report["summary"][outcome] != outcome_counts[test_outcome]:
                raise FaultEvidenceError(
                    f"Tier-1 {outcome} count does not match tests"
                )
        for test in tests:
            if len(test["attempts"]) != 1 or test["attempts"][0]["index"] != 1:
                raise FaultEvidenceError("killer test attempt census is invalid")
            attempt = test["attempts"][0]
            attempt_outcome = attempt["outcome"]
            failures = attempt["failures"]
            if any(failure["kind"] == "timeout" for failure in failures):
                raise FaultEvidenceError("killer test timed out")
            if attempt_outcome == "pass" and failures:
                raise FaultEvidenceError("passing killer has failure records")
            if attempt_outcome == "fail" and not failures:
                raise FaultEvidenceError("failed killer has no failure record")
            if test["outcome"] == "pass" and attempt_outcome != "pass":
                raise FaultEvidenceError("passing killer has a failed attempt")
            if test["outcome"] == "fail" and attempt_outcome != "fail":
                raise FaultEvidenceError("failed killer has no failed attempt")
        attempts_passed = sum(
            attempt["outcome"] == "pass"
            for test in tests
            for attempt in test["attempts"]
        )
        attempts_failed = sum(
            attempt["outcome"] == "fail"
            for test in tests
            for attempt in test["attempts"]
        )
        if report["summary"]["attempts_passed"] != attempts_passed or report["summary"][
            "attempts_failed"
        ] != attempts_failed:
            raise FaultEvidenceError("Tier-1 attempt summary does not match tests")
        failed = [test for test in tests if test["outcome"] == "fail"]
        if any(test["outcome"] in {"flaky", "skip"} for test in tests):
            raise FaultEvidenceError("flaky or skipped killer tests are invalid evidence")

        if baseline:
            if report["outcome"] != "pass" or failed:
                raise FaultEvidenceError("unmodified baseline is not green")
            return Attribution("baseline-green", (), ())

        if not failed:
            if report["outcome"] != "pass":
                raise FaultEvidenceError("passing killer tests conflict with report outcome")
            return Attribution("survived", (), ())
        if report["outcome"] != "fail":
            raise FaultEvidenceError("failed killer tests conflict with report outcome")
        if assertion_signature is None:
            raise FaultEvidenceError("fault attribution requires an assertion signature")
        matched = [
            test["name"]
            for test in failed
            if assertion_signature in _failure_text(test)
        ]
        if not matched:
            raise FaultEvidenceError("declared killer failed without its assertion signature")
        return Attribution(
            "killed",
            tuple(test["name"] for test in failed),
            (assertion_signature,),
        )
    except (FaultEvidenceError, QualificationValidationError, KeyError, TypeError) as error:
        return Attribution("invalid", (), (), str(error))


def verify_attribution_fixtures(fixtures: Path) -> None:
    baseline = _load_report(fixtures / "baseline-pass.json")
    killed = _load_report(fixtures / "fault-killed.json")
    expected = ["test_fixture_killer"]
    signature = "fixture assertion signature"
    if attribute_report(baseline, expected, "debug", None, baseline=True).outcome != "baseline-green":
        raise FaultEvidenceError("baseline attribution fixture")
    if attribute_report(killed, expected, "debug", signature).outcome != "killed":
        raise FaultEvidenceError("killed attribution fixture")
    if attribute_report(baseline, expected, "debug", signature).outcome != "survived":
        raise FaultEvidenceError("survived attribution fixture")

    incomplete = copy.deepcopy(killed)
    incomplete["complete"] = False
    incomplete["outcome"] = "incomplete"
    wrong_profile = copy.deepcopy(killed)
    wrong_profile["environment"]["build_profile"] = "release"
    wrong_selection = copy.deepcopy(killed)
    wrong_selection["selection"]["patterns"] = ["other"]
    wrong_signature = copy.deepcopy(killed)
    wrong_signature["tests"][0]["attempts"][0]["failures"][0]["message"] = "other failure"
    timeout = copy.deepcopy(killed)
    timeout["tests"][0]["attempts"][0]["failures"][0]["kind"] = "timeout"
    for document in (
        incomplete,
        wrong_profile,
        wrong_selection,
        wrong_signature,
        timeout,
    ):
        if attribute_report(document, expected, "debug", signature).outcome != "invalid":
            raise FaultEvidenceError("invalid attribution fixture was accepted")


def verify_exact_once_witness() -> None:
    source = (
        REPOSITORY / "itest" / "rust" / "src" / "event_bridge_tests.rs"
    ).read_text(encoding="utf-8")
    marker = "fn test_send_event_delivers_exactly_once("
    start = source.find(marker)
    if start < 0:
        raise FaultEvidenceError("missing exact-once event-bridge test")
    end = source.find("\n#[itest", start + len(marker))
    witness = source[start : end if end >= 0 else len(source)]
    if witness.count("app.update().await;") != 2:
        raise FaultEvidenceError("exact-once witness must observe two frames")
    if "received, 1" not in witness or "exactly once across two frames" not in witness:
        raise FaultEvidenceError("exact-once witness does not assert observer count one")


def _git(cwd: Path, *arguments: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["git", *arguments],
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if check and result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise FaultEvidenceError(f"git {' '.join(arguments)} failed: {detail}")
    return result


def _environment() -> dict[str, Any]:
    commit = _git(REPOSITORY, "rev-parse", "HEAD").stdout.strip()
    dirty = bool(
        _git(REPOSITORY, "status", "--porcelain", "--untracked-files=all").stdout.strip()
    )
    return {
        "git_commit": commit,
        "git_dirty": dirty,
        "os": platform.system().lower(),
        "arch": platform.machine(),
        "cargo_profile": None,
        "tools": {},
        "metadata": {},
    }


def _write_report(run_dir: Path, document: dict[str, Any]) -> Path:
    document["generated_at"] = utc_now()
    path = run_dir / "qualification-v1.json"
    write_qualification(path, document, QUALIFICATION_SCHEMA)
    latest = REPOSITORY / "target" / "qualification" / "latest-faults" / path.name
    latest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(path, latest)
    return path


def _kill_process_group(process: subprocess.Popen[Any]) -> None:
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    except OSError:
        process.kill()
    process.wait()


def _run_itest(
    scratch: Path,
    run_dir: Path,
    label: str,
    profile: str,
    tests: list[str],
    target_dir: Path,
) -> tuple[int, Path, Path]:
    target_dir.mkdir(parents=True, exist_ok=True)
    target_link = scratch / "target"
    if target_link.is_symlink():
        if target_link.resolve() != target_dir.resolve():
            target_link.unlink()
    elif target_link.exists():
        raise FaultEvidenceError("scratch target path is not a qualification symlink")
    if not target_link.exists():
        target_link.symlink_to(target_dir, target_is_directory=True)

    report = run_dir / "tier1" / f"{label}-{profile}.json"
    log = run_dir / "logs" / f"{label}-{profile}.log"
    report.parent.mkdir(parents=True, exist_ok=True)
    log.parent.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment.update(
        {
            "CARGO_TARGET_DIR": str(target_link),
            "ITEST_FILTER": ",".join(tests),
            "ITEST_JSON_PATH": str(report),
            "ITEST_DENY_FOCUS": "1",
            "ITEST_REPEAT": "1",
            "ITEST_TIMEOUT_FRAMES": "600",
        }
    )
    command = [str(scratch / "itest" / "run-tests.sh")]
    if profile == "release":
        command.append("--release")
    with log.open("w", encoding="utf-8") as handle:
        handle.write("command: " + " ".join(command) + "\n")
        handle.flush()
        process: subprocess.Popen[Any] | None = None
        try:
            process = subprocess.Popen(
                command,
                cwd=scratch,
                env=environment,
                stdout=handle,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            return_code = process.wait(timeout=ITEST_TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired as error:
            if process is not None:
                _kill_process_group(process)
            raise FaultEvidenceError(f"{label} [{profile}] timed out") from error
        except BaseException:
            if process is not None:
                _kill_process_group(process)
            raise
    return return_code, report, log


def _record_artifacts(
    document: dict[str, Any], label: str, profile: str, report: Path, log: Path
) -> None:
    document["artifacts"].extend(
        [
            artifact_record(f"tier1-{label}-{profile}", report, REPOSITORY),
            artifact_record(f"log-{label}-{profile}", log, REPOSITORY),
        ]
    )


def _apply_fault(scratch: Path, fault: Fault) -> None:
    _patch_paths(fault.patch)
    check = _git(scratch, "apply", "--check", str(fault.patch), check=False)
    if check.returncode != 0:
        raise FaultEvidenceError(f"{fault.id} apply check failed: {check.stderr.strip()}")
    _git(scratch, "apply", str(fault.patch))


def _reverse_fault(scratch: Path, fault: Fault) -> None:
    reverse = _git(
        scratch, "apply", "--reverse", "--check", str(fault.patch), check=False
    )
    if reverse.returncode != 0:
        raise FaultEvidenceError(f"{fault.id} reverse check failed: {reverse.stderr.strip()}")
    _git(scratch, "apply", "--reverse", str(fault.patch))
    clean = _git(scratch, "diff", "--exit-code", check=False)
    if clean.returncode != 0:
        raise FaultEvidenceError(f"{fault.id} reverse left a dirty worktree")


def _fault_record(
    fault: Fault,
    profile: str,
    attribution: Attribution,
    report: Path | None,
) -> dict[str, Any]:
    try:
        report_path = report.resolve().relative_to(REPOSITORY.resolve()).as_posix() if report else None
    except ValueError:
        report_path = str(report.resolve()) if report else None
    return {
        "id": fault.id,
        "profile": profile,
        "outcome": attribution.outcome,
        "killer_tests": [fault.killer_test],
        "failed_tests": list(attribution.failed_tests),
        "matched_signatures": list(attribution.matched_signatures),
        "report_path": report_path,
        "metadata": {"error": attribution.error} if attribution.error else {},
    }


def run_fault_pack(profiles: list[str]) -> int:
    manifest = load_manifest()
    if profiles != list(manifest.profiles):
        print("ERROR fault-pack: profiles must be debug,release")
        return 2
    environment = _environment()
    document = new_document("fault-pack", "faults", environment)
    run_dir = OUTPUT_ROOT / document["run_id"]
    run_dir.mkdir(parents=True, exist_ok=False)
    scratch = (
        REPOSITORY
        / "target"
        / "qualification-worktrees"
        / "faults"
        / document["run_id"]
    )
    scratch.parent.mkdir(parents=True, exist_ok=True)
    added_worktree = False
    invalid_errors: list[str] = []
    baseline_green: list[str] = []
    results: dict[str, dict[str, Attribution]] = {
        fault.id: {} for fault in manifest.faults
    }
    try:
        if environment["git_dirty"]:
            raise FaultEvidenceError(
                "live fault qualification requires committed machinery and a clean worktree"
            )
        verify_patch_applicability(manifest)
        _git(REPOSITORY, "worktree", "add", "--detach", str(scratch), "HEAD")
        added_worktree = True
        target_root = (
            REPOSITORY
            / "target"
            / "qualification-cargo"
            / "faults"
            / environment["git_commit"]
        )
        targets = {profile: target_root / profile for profile in profiles}

        union = [fault.killer_test for fault in manifest.faults]
        for profile in profiles:
            code, report_path, log = _run_itest(
                scratch,
                run_dir,
                "baseline",
                profile,
                union,
                targets[profile],
            )
            _record_artifacts(document, "baseline", profile, report_path, log)
            if code != 0:
                raise FaultEvidenceError(
                    f"unmodified baseline [{profile}] exited {code}"
                )
            attribution = attribute_report(
                _load_report(report_path), union, profile, None, baseline=True
            )
            if attribution.outcome != "baseline-green":
                raise FaultEvidenceError(
                    f"unmodified baseline [{profile}] invalid: {attribution.error}"
                )
            if _git(scratch, "diff", "--exit-code", check=False).returncode != 0:
                raise FaultEvidenceError(
                    f"unmodified baseline [{profile}] changed tracked files"
                )
            baseline_green.append(profile)

        for fault in manifest.faults:
            for profile in profiles:
                applied = False
                revert_error: BaseException | None = None
                report_path: Path | None = None
                attribution = Attribution("invalid", (), (), "fault did not run")
                try:
                    _apply_fault(scratch, fault)
                    applied = True
                    code, report_path, log = _run_itest(
                        scratch,
                        run_dir,
                        fault.id,
                        profile,
                        [fault.killer_test],
                        targets[profile],
                    )
                    _record_artifacts(document, fault.id, profile, report_path, log)
                    attribution = attribute_report(
                        _load_report(report_path),
                        [fault.killer_test],
                        profile,
                        fault.assertion_signature,
                    )
                    expected_exit = 1 if attribution.outcome == "killed" else 0
                    if attribution.outcome == "invalid" or code != expected_exit:
                        reason = attribution.error or (
                            f"Tier-1 exit {code}, expected {expected_exit} for {attribution.outcome}"
                        )
                        attribution = Attribution("invalid", (), (), reason)
                except BaseException as error:
                    attribution = Attribution("invalid", (), (), str(error))
                finally:
                    if applied:
                        try:
                            _reverse_fault(scratch, fault)
                        except BaseException as error:
                            revert_error = error
                            attribution = Attribution("invalid", (), (), str(error))
                results[fault.id][profile] = attribution
                document["faults"].append(
                    _fault_record(fault, profile, attribution, report_path)
                )
                if attribution.outcome == "invalid":
                    invalid_errors.append(
                        f"{fault.id} [{profile}]: {attribution.error or 'invalid evidence'}"
                    )
                if revert_error is not None:
                    raise FaultEvidenceError(str(revert_error))
    except BaseException as error:
        invalid_errors.append(str(error))
    finally:
        if added_worktree:
            removed = _git(
                REPOSITORY,
                "worktree",
                "remove",
                "--force",
                str(scratch),
                check=False,
            )
            if removed.returncode != 0:
                invalid_errors.append(
                    f"scratch worktree cleanup failed: {removed.stderr.strip()}"
                )

    survivors = sum(
        attribution.outcome == "survived"
        for profile_results in results.values()
        for attribution in profile_results.values()
    )
    invalid_records = sum(
        attribution.outcome == "invalid"
        for profile_results in results.values()
        for attribution in profile_results.values()
    )
    invalid = max(invalid_records, 1 if invalid_errors else 0)
    killed = sum(
        attribution.outcome == "killed"
        for profile_results in results.values()
        for attribution in profile_results.values()
    )
    document["summary"] = {
        "total": len(document["faults"]),
        "passed": killed,
        "failed": survivors,
        "invalid": invalid,
        "skipped": 0,
        "counts": {
            "faults": len(manifest.faults),
            "profile_kills": killed,
            "survivors": survivors,
            "invalid": invalid,
        },
    }
    document["metadata"] = {
        "baseline_green_profiles": baseline_green,
        "profiles": profiles,
        "shared_target_root": (
            f"target/qualification-cargo/faults/{environment['git_commit']}"
        ),
    }
    if invalid_errors:
        document["complete"] = False
        document["outcome"] = "error"
        document["errors"] = [
            error_record("invalid-evidence", message, "fault-pack")
            for message in invalid_errors
        ]
        exit_code = 2
    else:
        document["complete"] = True
        document["outcome"] = "fail" if survivors else "pass"
        exit_code = 1 if survivors else 0
    _write_report(run_dir, document)

    for fault in manifest.faults:
        profile_results = results[fault.id]
        if all(
            profile_results.get(profile, Attribution("invalid", (), ())).outcome
            == "killed"
            for profile in profiles
        ):
            print(
                f"KILLED {fault.id} [debug,release] by {fault.killer_test}"
            )
    if exit_code == 0:
        print(
            "PASS fault-pack: baseline=green faults=12 profile-kills=24 "
            "survivors=0 invalid=0"
        )
    elif exit_code == 1:
        print(f"FAIL fault-pack: survivors={survivors}")
    else:
        print(f"ERROR fault-pack: incomplete evidence invalid={invalid}")
    return exit_code


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profiles", required=True)
    arguments = parser.parse_args()
    profiles = [profile.strip() for profile in arguments.profiles.split(",") if profile.strip()]
    try:
        return run_fault_pack(profiles)
    except BaseException as error:
        print(f"ERROR fault-pack: {error}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
