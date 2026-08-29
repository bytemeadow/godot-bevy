#!/usr/bin/env python3
from __future__ import annotations

import copy
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Callable

from qualification_schema import (
    QualificationValidationError,
    qualification_exit,
    validate_qualification,
)
from qualification_toml import TomlError, load_toml, loads_toml

REPOSITORY = Path(__file__).resolve().parents[1]
SCHEMA = REPOSITORY / "godot-bevy-test" / "schema" / "qualification-v1.schema.json"
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "qualification"
TERMINAL_FIXTURES = FIXTURES / "terminal"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(value, dict), f"{path}: fixture root must be an object")
    return value


def rejected(document: dict[str, Any], *, require_complete: bool = False) -> None:
    try:
        validate_qualification(document, SCHEMA, require_complete=require_complete)
    except QualificationValidationError:
        return
    raise AssertionError("invalid qualification document was accepted")


def verify_contract() -> None:
    documents = {
        path.stem: load_json(path)
        for path in sorted(TERMINAL_FIXTURES.glob("*.json"))
    }
    require(
        set(documents) == {"error", "fail", "incomplete", "pass", "skip"},
        "terminal fixture census",
    )
    for document in documents.values():
        validate_qualification(document, SCHEMA)
    validate_qualification(documents["pass"], SCHEMA, require_complete=True)
    validate_qualification(documents["fail"], SCHEMA, require_complete=True)
    validate_qualification(documents["skip"], SCHEMA, require_complete=True)
    print("PASS qualification-v1 schema")

    exits = {name: qualification_exit(document) for name, document in documents.items()}
    require(
        exits == {"error": 2, "fail": 1, "incomplete": 2, "pass": 0, "skip": 0},
        f"qualification exit mapping: {exits}",
    )
    print("PASS qualification exit mapping")

    invalid = copy.deepcopy(documents["pass"])
    invalid["unexpected"] = True
    rejected(invalid)

    invalid = copy.deepcopy(documents["pass"])
    invalid["environment"]["unexpected"] = True
    rejected(invalid)

    invalid = copy.deepcopy(documents["pass"])
    invalid["complete"] = False
    rejected(invalid)

    invalid = copy.deepcopy(documents["incomplete"])
    invalid["complete"] = True
    rejected(invalid)

    invalid = copy.deepcopy(documents["pass"])
    invalid["checks"].append(copy.deepcopy(invalid["checks"][0]))
    invalid["summary"]["total"] = 2
    invalid["summary"]["passed"] = 2
    rejected(invalid)

    invalid = copy.deepcopy(documents["pass"])
    invalid["checks"][0]["outcome"] = "fail"
    invalid["checks"][0]["exit_code"] = 1
    invalid["summary"]["passed"] = 0
    invalid["summary"]["failed"] = 1
    rejected(invalid)

    invalid = copy.deepcopy(documents["pass"])
    invalid["checks"][0]["exit_code"] = 1
    rejected(invalid)

    invalid = copy.deepcopy(documents["error"])
    invalid["complete"] = True
    rejected(invalid)

    invalid = copy.deepcopy(documents["pass"])
    invalid["evidence_kind"] = "mutation-run"
    invalid["checks"] = []
    invalid["mutants"] = [
        {
            "id": "sha256:" + "0" * 64,
            "package": "fixture",
            "path": "src/lib.rs",
            "function": None,
            "genre": "FnValue",
            "replacement": "false",
            "source_slice_sha256": "0" * 64,
            "outcome": "missed",
            "name": "fixture mutant",
            "location": {"line": 1, "column": 1},
            "log_path": None,
            "diff_path": None,
            "metadata": {},
        }
    ]
    invalid["summary"].update({"passed": 0, "failed": 1})
    rejected(invalid)

    invalid = copy.deepcopy(documents["pass"])
    invalid["evidence_kind"] = "fault-pack"
    invalid["checks"] = []
    invalid["faults"] = [
        {
            "id": "fixture_fault",
            "profile": "debug",
            "outcome": "survived",
            "killer_tests": ["test_fixture"],
            "failed_tests": [],
            "matched_signatures": [],
            "report_path": None,
            "metadata": {},
        }
    ]
    invalid["summary"].update({"passed": 0, "failed": 1})
    rejected(invalid)

    extension = copy.deepcopy(documents["pass"])
    extension["metadata"]["fixture-extension"] = {"allowed": True}
    extension["artifacts"][0]["metadata"]["fixture-extension"] = [1, 2, 3]
    validate_qualification(extension, SCHEMA, require_complete=True)

    for name in ("error", "incomplete"):
        rejected(documents[name], require_complete=True)

    schema = load_json(SCHEMA)
    schema["allOf"] = []
    with tempfile.TemporaryDirectory() as temporary:
        unsupported_schema = Path(temporary) / "schema.json"
        unsupported_schema.write_text(json.dumps(schema), encoding="utf-8")
        try:
            validate_qualification(documents["pass"], unsupported_schema)
        except QualificationValidationError:
            pass
        else:
            raise AssertionError("unsupported schema keyword was ignored")

    parsed_toml = loads_toml(
        'version = 1\n[[checks]]\nid = "fixture"\ncommand = ["true"]\n'
    )
    require(parsed_toml["checks"][0]["id"] == "fixture", "TOML parser fixture")
    for invalid_toml in ('value = ["unterminated"\n', "value = unsupported\n"):
        try:
            loads_toml(invalid_toml)
        except TomlError:
            pass
        else:
            raise AssertionError("invalid TOML fixture was accepted")
    print("PASS qualification fail-closed fixtures")


def verify_faults_fixtures() -> None:
    from qualification_faults import verify_attribution_fixtures

    verify_attribution_fixtures(FIXTURES / "faults")
    print("PASS fault attribution fixtures")


def verify_faults() -> None:
    from qualification_faults import (
        load_manifest,
        verify_attribution_fixtures,
        verify_exact_once_witness,
        verify_patch_applicability,
    )

    manifest = load_manifest()
    print("PASS fault manifest: faults=12 profiles=debug,release")
    applicable, reversible = verify_patch_applicability(manifest)
    require((applicable, reversible) == (12, 12), "fault patch census")
    print("PASS fault patches: applicable=12 reversible=12")
    verify_attribution_fixtures(FIXTURES / "faults")
    print("PASS fault attribution fixtures")
    verify_exact_once_witness()
    print("PASS event-bridge exact-once witness")


def verify_all_fixtures() -> None:
    verify_contract()
    verify_mutants_fixtures()
    verify_faults_fixtures()


def verify_workflow() -> None:
    from qualification import AggregateError, load_registry

    workflow = (
        REPOSITORY / ".github" / "workflows" / "qualification.yml"
    ).read_text(encoding="utf-8")
    require("\non:\n" in workflow and "\nenv:\n" in workflow, "qualification trigger block")
    trigger_block = workflow.split("\non:\n", 1)[1].split("\nenv:\n", 1)[0]
    triggers = re.findall(r"^  ([A-Za-z_][A-Za-z0-9_-]*):", trigger_block, re.MULTILINE)
    require(triggers == ["workflow_dispatch"], f"qualification workflow triggers: {triggers}")
    require(
        all(f"          - {suite}\n" in workflow for suite in ("mutants", "faults", "both")),
        "qualification workflow suite choices",
    )
    require("runs-on: ubuntu-latest" in workflow, "qualification workflow is not Linux")
    require('ITEST_DENY_FOCUS: "1"' in workflow, "qualification workflow permits focus")
    print("PASS qualification workflow: workflow_dispatch-only")

    require("uses: actions/upload-artifact@v4" in workflow, "qualification artifact upload")
    upload = workflow.index("uses: actions/upload-artifact@v4")
    preceding = workflow[max(0, upload - 160) : upload]
    require("if: always()" in preceding, "qualification artifacts are not always uploaded")
    print("PASS qualification workflow: artifacts-always-uploaded")

    ci = (REPOSITORY / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
    require("qualification-static:" in ci, "ordinary CI lacks qualification static job")
    require("needs: [changes]" in ci, "qualification static job bypasses changes filter")
    require(
        "./itest/verify-qualification.sh contract" in ci
        and "./itest/verify-qualification.sh faults" in ci,
        "ordinary CI lacks cheap qualification checks",
    )
    checks = load_registry()
    require([check.id for check in checks] == ["mutants", "fault-pack"], "check registry seam")
    with tempfile.TemporaryDirectory() as temporary:
        for index, artifact in enumerate(("../escape.json", "Cargo.toml")):
            invalid_registry = Path(temporary) / f"checks-{index}.toml"
            invalid_registry.write_text(
                'version = 1\n[[checks]]\nid = "bad"\n'
                f'command = ["true"]\nartifact = "{artifact}"\n'
                'evidence_kind = "aggregate"\n',
                encoding="utf-8",
            )
            try:
                load_registry(invalid_registry)
            except AggregateError:
                pass
            else:
                raise AssertionError("unsafe aggregate registry was accepted")
    print("PASS qualification workflow: static-check-in-ci")


def verify_mutants_fixtures() -> None:
    from qualification_compare import compare_mutant_outcomes
    from qualification_mutants import (
        MutationEvidenceError,
        _candidate_count,
        _diff_has_production_changes,
        _missing_viable_scope,
        _source_slice,
        diff_decision,
        normalize_outcomes,
    )

    source = FIXTURES / "mutants-source"
    output = FIXTURES / "mutants.out"
    run = normalize_outcomes(output, source)
    require(run.version == "27.1.0", "cargo-mutants fixture version")
    require(run.baseline_passed, "cargo-mutants baseline fixture")
    require(
        [mutant.outcome for mutant in run.mutants]
        == ["caught", "missed", "timeout", "unviable"],
        "cargo-mutants outcome normalization",
    )
    require(
        all(mutant.path == "godot-bevy/src/fixture.rs" for mutant in run.mutants),
        "cargo-mutants path normalization",
    )
    require(
        run.mutants[0].source_slice == b"+"
        and run.mutants[1].source_slice == b"!value"
        and run.mutants[3].source_slice == b"1",
        "cargo-mutants source span extraction",
    )
    require(
        len({mutant.stable_id for mutant in run.mutants}) == 4,
        "stable mutant identities are unique",
    )
    fixture_path = "godot-bevy/src/fixture.rs"
    require(
        _missing_viable_scope(run.mutants, [fixture_path]) == [],
        "viable mutation scope fixture",
    )
    require(
        _missing_viable_scope([run.mutants[-1]], [fixture_path]) == [fixture_path],
        "inert mutation scope fixture",
    )
    with tempfile.TemporaryDirectory() as temporary:
        unicode_source = Path(temporary) / "unicode.rs"
        unicode_source.write_bytes("αβ+\r\n".encode("utf-8"))
        source_slice, line, column = _source_slice(
            unicode_source,
            {
                "start": {"line": 1, "column": 3},
                "end": {"line": 1, "column": 4},
            },
        )
        require(
            (source_slice, line, column) == (b"+", 1, 3),
            "cargo-mutants one-based character span",
        )
    first_identity = run.mutants[0].stable_id
    shifted = copy.deepcopy(json.loads((output / "outcomes.json").read_text()))
    shifted_source = (
        source / "godot-bevy" / "src" / "fixture.rs"
    ).read_text(encoding="utf-8")
    shifted["outcomes"][1]["scenario"]["Mutant"]["span"]["start"]["line"] += 1
    shifted["outcomes"][1]["scenario"]["Mutant"]["span"]["end"]["line"] += 1
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        shifted_output = root / "mutants.out"
        shifted_source_root = root / "source"
        (shifted_output / "logs").mkdir(parents=True)
        (shifted_output / "diff").mkdir()
        (shifted_source_root / "godot-bevy" / "src").mkdir(parents=True)
        (shifted_output / "outcomes.json").write_text(
            json.dumps(shifted), encoding="utf-8"
        )
        for path in (output / "logs").iterdir():
            (shifted_output / "logs" / path.name).write_bytes(path.read_bytes())
        for path in (output / "diff").iterdir():
            (shifted_output / "diff" / path.name).write_bytes(path.read_bytes())
        (shifted_source_root / "godot-bevy" / "src" / "fixture.rs").write_text(
            "\n" + shifted_source, encoding="utf-8"
        )
        shifted_run = normalize_outcomes(shifted_output, shifted_source_root)
        require(
            shifted_run.mutants[0].stable_id == first_identity,
            "line metadata must not change stable identity",
        )

    def normalization_rejects(mutator: Callable[[dict[str, Any]], None]) -> None:
        document = json.loads((output / "outcomes.json").read_text(encoding="utf-8"))
        mutator(document)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "mutants.out"
            path.mkdir()
            (path / "outcomes.json").write_text(json.dumps(document), encoding="utf-8")
            shutil.copytree(output / "logs", path / "logs")
            shutil.copytree(output / "diff", path / "diff")
            try:
                normalize_outcomes(path, source)
            except MutationEvidenceError:
                return
        raise AssertionError("invalid cargo-mutants evidence was accepted")

    normalization_rejects(
        lambda document: document.__setitem__("cargo_mutants_version", "27.0.0")
    )
    normalization_rejects(
        lambda document: document.__setitem__("total_mutants", 99)
    )
    normalization_rejects(lambda document: document.__setitem__("end_time", None))
    normalization_rejects(
        lambda document: document["outcomes"][1]["scenario"]["Mutant"].__setitem__(
            "file", "../outside.rs"
        )
    )
    normalization_rejects(
        lambda document: document["outcomes"][1].__setitem__("summary", "Failure")
    )
    normalization_rejects(
        lambda document: document["outcomes"][1].__setitem__("phase_results", [])
    )
    normalization_rejects(
        lambda document: document["outcomes"][1]["phase_results"][0].__setitem__(
            "process_status", "Success"
        )
    )
    normalization_rejects(
        lambda document: document["outcomes"][1]["phase_results"].insert(
            0,
            {
                "phase": "Build",
                "duration": 0.1,
                "process_status": {"Failure": 101},
                "argv": ["cargo", "test", "--no-run"],
            },
        )
    )
    normalization_rejects(
        lambda document: document["outcomes"][2]["phase_results"].insert(
            0,
            {
                "phase": "Build",
                "duration": 0.1,
                "process_status": "Timeout",
                "argv": ["cargo", "test", "--no-run"],
            },
        )
    )
    normalization_rejects(
        lambda document: document["outcomes"][0]["phase_results"].insert(
            0,
            {
                "phase": "Build",
                "duration": 0.1,
                "process_status": "Timeout",
                "argv": ["cargo", "test", "--no-run"],
            },
        )
    )

    regressions, new_ids = compare_mutant_outcomes(
        {"caught-now-missed": "caught", "removed": "caught"},
        {"caught-now-missed": "missed", "new": "caught"},
    )
    require(regressions == ["caught-now-missed"], "identity regression comparison")
    require(new_ids == ["new"], "new identity comparison")
    require(_candidate_count([{"name": "one"}]) == 1, "candidate list shape")
    for invalid_candidates in (
        None,
        {},
        {"mutants": [{"name": "one"}]},
        ["not-an-object"],
        [{}],
    ):
        try:
            _candidate_count(invalid_candidates)
        except MutationEvidenceError:
            pass
        else:
            raise AssertionError("invalid candidate JSON was accepted")

    require(diff_decision([], [], 0) == "skip", "out-of-scope diff skip")
    require(
        diff_decision(
            ["godot-bevy-test/src/selection.rs"],
            [],
            0,
            production_changed=False,
        )
        == "skip",
        "test-only scoped diff skip",
    )
    require(
        diff_decision(["godot-bevy/src/plugins/event_bridge.rs"], [], 1) == "run",
        "scoped diff run",
    )
    for changed, untracked, candidates in (
        (["godot-bevy/src/plugins/event_bridge.rs"], [], 0),
        ([], ["godot-bevy-test/src/selection.rs"], 0),
    ):
        try:
            diff_decision(changed, untracked, candidates)
        except MutationEvidenceError:
            pass
        else:
            raise AssertionError("invalid diff evidence was accepted")

    old_source = "fn production() {}\n\n#[cfg(test)]\nmod tests {\n    assert!(true);\n}\n"
    new_source = old_source.replace("assert!(true)", "assert!(false)")
    test_diff = "@@ -5 +5 @@\n-    assert!(true);\n+    assert!(false);\n"
    require(
        not _diff_has_production_changes(test_diff, old_source, new_source),
        "test-only diff classification",
    )
    production_diff = "@@ -1 +1 @@\n-fn production() {}\n+fn production() { panic!() }\n"
    require(
        _diff_has_production_changes(production_diff, old_source, new_source),
        "production diff classification",
    )
    old_after_tests = old_source + "fn after_tests() {}\n"
    new_after_tests = old_source + "fn after_tests() { panic!() }\n"
    after_tests_diff = (
        "@@ -7 +7 @@\n"
        "-fn after_tests() {}\n"
        "+fn after_tests() { panic!() }\n"
    )
    require(
        _diff_has_production_changes(
            after_tests_diff,
            old_after_tests,
            new_after_tests,
        ),
        "production after test module classification",
    )
    print("PASS mutants normalization and identity fixtures")
    print("PASS mutants diff and exit fixtures")


def verify_mutants(*, include_tool: bool = True) -> None:
    from qualification_mutants import EXPECTED_SCOPE

    expected_scope = EXPECTED_SCOPE
    config_path = REPOSITORY / ".cargo" / "mutants.toml"
    config = load_toml(config_path)
    require(
        config
        == {
            "additional_cargo_args": ["--locked"],
            "test_workspace": False,
            "timeout_multiplier": 3.0,
            "minimum_test_timeout": 20.0,
            "examine_globs": expected_scope,
        },
        "mutation config differs from the converged positive-only scope",
    )
    require(
        all((REPOSITORY / path).is_file() for path in expected_scope),
        "mutation scope contains a missing file",
    )
    devenv = (REPOSITORY / "devenv.nix").read_text(encoding="utf-8")
    require("cargo-mutants" in devenv, "devenv does not include cargo-mutants")

    if include_tool:
        executable = shutil.which("cargo-mutants")
        require(executable is not None, "missing cargo-mutants")
        resolved = str(Path(executable).resolve())
        require(
            re.search(r"cargo-mutants-27\.1\.0(?:/|$)", resolved) is not None,
            f"cargo-mutants is not the nixpkgs 27.1.0 package: {resolved}",
        )
        version = subprocess.run(
            [executable, "mutants", "--version"],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        require(
            version.returncode == 0
            and version.stdout.strip() == "cargo-mutants 27.1.0",
            f"cargo-mutants version output: {version.stdout.strip()}",
        )
        print("PASS mutants tool: cargo-mutants=27.1.0")
    print("PASS mutants scope: files=15 positive-only=true")
    verify_mutants_fixtures()


MODES: dict[str, Callable[[], None]] = {
    "contract": verify_contract,
    "fixtures": verify_all_fixtures,
    "mutants-fixtures": verify_mutants_fixtures,
    "mutants": verify_mutants,
    "mutants-static": lambda: verify_mutants(include_tool=False),
    "faults-fixtures": verify_faults_fixtures,
    "faults": verify_faults,
    "workflow": verify_workflow,
}


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in MODES:
        print(
            "usage: test_qualification.py " + "|".join(sorted(MODES)),
            file=sys.stderr,
        )
        return 2
    MODES[sys.argv[1]]()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
