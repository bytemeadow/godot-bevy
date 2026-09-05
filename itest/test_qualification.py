#!/usr/bin/env python3
from __future__ import annotations

import copy
import hashlib
import json
import platform
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
SCHEMA = REPOSITORY / "itest" / "schema" / "qualification-v1.schema.json"
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "qualification"
TERMINAL_FIXTURES = FIXTURES / "terminal"
DOCTEST_LEDGER = REPOSITORY / "itest" / "qualification" / "doctests-v1.toml"
DOCTEST_ARTIFACT = (
    REPOSITORY / "target" / "qualification" / "latest-doctests" / "qualification-v1.json"
)
ASSERTION_LEDGER = REPOSITORY / "itest" / "qualification" / "assertions-v1.toml"
ASSERTION_ARTIFACT = (
    REPOSITORY / "target" / "qualification" / "latest-assertions" / "qualification-v1.json"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(value, dict), f"{path}: fixture root must be an object")
    return value


def write_audit_evidence(
    evidence_kind: str,
    check_id: str,
    artifact: Path,
    counts: dict[str, int],
) -> None:
    from qualification_schema import new_document, utc_now, write_qualification

    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=REPOSITORY,
        text=True,
        stdout=subprocess.PIPE,
        check=True,
    ).stdout.strip()
    dirty = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=all"],
        cwd=REPOSITORY,
        text=True,
        stdout=subprocess.PIPE,
        check=True,
    ).stdout.strip()
    document = new_document(
        evidence_kind,
        "full",
        {
            "git_commit": commit,
            "git_dirty": bool(dirty),
            "os": platform.system().lower(),
            "arch": platform.machine(),
            "cargo_profile": None,
            "tools": {},
            "metadata": {},
        },
    )
    document["checks"] = [
        {
            "id": check_id,
            "outcome": "pass",
            "exit_code": 0,
            "artifact": None,
            "metadata": counts,
        }
    ]
    document["summary"] = {
        "total": 1,
        "passed": 1,
        "failed": 0,
        "invalid": 0,
        "skipped": 0,
        "counts": counts,
    }
    document["complete"] = True
    document["outcome"] = "pass"
    document["generated_at"] = utc_now()
    write_qualification(artifact, document, SCHEMA)


def rustdoc_fences() -> list[dict[str, Any]]:
    roots = (
        REPOSITORY / "godot-bevy" / "src",
        REPOSITORY / "godot-bevy-macros" / "src",
        REPOSITORY / "godot-bevy-test" / "src",
        REPOSITORY / "godot-bevy-test-macros" / "src",
    )
    fences: list[dict[str, Any]] = []
    for path in sorted(file for root in roots for file in root.rglob("*.rs")):
        lines = path.read_text(encoding="utf-8").splitlines()
        ordinal = 0
        index = 0
        while index < len(lines):
            opening = re.match(
                r"^\s*//[/!]\s*```(?:rust,)?(ignore|no_run)\s*$", lines[index]
            )
            if opening is None:
                index += 1
                continue
            ordinal += 1
            kind = opening.group(1)
            code: list[str] = []
            cursor = index + 1
            while cursor < len(lines):
                doc = re.match(r"^\s*//[/!] ?(.*)$", lines[cursor])
                require(doc is not None, f"{path}:{cursor + 1}: unterminated doc fence")
                if doc.group(1).strip() == "```":
                    break
                code.append(doc.group(1))
                cursor += 1
            require(cursor < len(lines), f"{path}:{index + 1}: unterminated doc fence")
            nearby = "\n".join(lines[max(0, index - 4) : index])
            marker = re.search(r"qualification-doctest: scaffold=([^\s]+)", nearby)
            reason = re.search(
                r"qualification-doctest:.*\breason=([a-z0-9-]+)", nearby
            )
            fences.append(
                {
                    "source": path.relative_to(REPOSITORY).as_posix(),
                    "ordinal": ordinal,
                    "kind": kind,
                    "fingerprint": "sha256:"
                    + hashlib.sha256("\n".join(code).encode()).hexdigest(),
                    "scaffold": marker.group(1) if marker else None,
                    "reason": reason.group(1) if reason else None,
                }
            )
            index = cursor + 1
    return fences


def verify_doctests() -> None:
    ledger = load_toml(DOCTEST_LEDGER)
    require(
        set(ledger) == {"version", "scaffold_path", "scaffold_sha256", "fences"},
        "doctest ledger fields",
    )
    require(ledger["version"] == 1, "doctest ledger version")
    scaffold_path = REPOSITORY / ledger["scaffold_path"]
    require(scaffold_path.is_file(), "doctest scaffold missing")
    scaffold_text = scaffold_path.read_text(encoding="utf-8")
    require(
        "sha256:" + hashlib.sha256(scaffold_text.encode()).hexdigest()
        == ledger["scaffold_sha256"],
        "doctest scaffold hash is stale",
    )

    actual = rustdoc_fences()
    expected = ledger["fences"]
    require(isinstance(expected, list), "doctest fence ledger must be an array")
    require(len(actual) == len(expected) == 42, "doctest fence census")
    actual_by_key = {(item["source"], item["ordinal"]): item for item in actual}
    require(len(actual_by_key) == len(actual), "duplicate doctest fence identity")
    expected_keys = [(item.get("source"), item.get("ordinal")) for item in expected]
    require(
        len(set(expected_keys)) == len(expected_keys),
        "duplicate doctest ledger identity",
    )
    ignored = 0
    for entry in expected:
        require(
            isinstance(entry, dict)
            and set(entry) == {
                "source",
                "ordinal",
                "category",
                "fingerprint",
                "scaffold",
            },
            "invalid doctest ledger entry",
        )
        key = (entry["source"], entry["ordinal"])
        require(key in actual_by_key, f"missing doctest fence {key}")
        fence = actual_by_key[key]
        require(
            fence["fingerprint"] == entry["fingerprint"],
            f"stale doctest fence {key}",
        )
        category = entry["category"]
        require(
            category in {"compiled_no_run", "compiled_scaffold", "justified_ignore"},
            f"invalid doctest category {key}",
        )
        if category == "compiled_no_run":
            require(fence["kind"] == "no_run", f"unchecked ignore {key}")
            require(entry["scaffold"] == "", f"unexpected scaffold {key}")
        else:
            ignored += 1
            require(fence["kind"] == "ignore", f"non-ignore ledger category {key}")
            require(entry["scaffold"], f"ignored fence lacks reference {key}")
            require(fence["scaffold"] == entry["scaffold"], f"scaffold marker drift {key}")
            if category == "justified_ignore":
                require(fence["reason"], f"ignored fragment lacks reason {key}")
            path_text, anchor = entry["scaffold"].split("#", 1)
            require(path_text == ledger["scaffold_path"], f"unexpected scaffold path {key}")
            require(f"mod {anchor} {{" in scaffold_text, f"missing scaffold anchor {anchor}")
    require(ignored == 13, "doctest scaffold census")
    doctests = subprocess.run(
        [
            "cargo",
            "test",
            "--doc",
            "-p",
            "godot-bevy",
            "-p",
            "godot-bevy-macros",
            "-p",
            "godot-bevy-test",
            "-p",
            "godot-bevy-test-macros",
        ],
        cwd=REPOSITORY,
        check=False,
    )
    require(doctests.returncode == 0, "published-crate doctests failed")
    scaffold = subprocess.run(
        ["cargo", "check", "-p", "book-tests", "--all-targets"],
        cwd=REPOSITORY,
        check=False,
    )
    require(scaffold.returncode == 0, "doctest scaffold compilation failed")
    write_audit_evidence(
        "doctest-audit",
        "doctests",
        DOCTEST_ARTIFACT,
        {
            "fences": len(expected),
            "compiled_no_run": len(expected) - ignored,
            "compiled_scaffold": ignored,
        },
    )
    print("PASS doctest policy: unchecked-ignore=0")
    print("PASS doctest scaffolds: synchronized")
    print("PASS doctest compilation: published-crates=4")


APPROXIMATE_EXPRESSION = re.compile(
    r"\.abs\(\)|\b(?:EPSILON|TOLERANCE)\b|"
    r"\b(?:assert_[A-Za-z0-9_]*(?:near|approx|relative|ulps|abs_diff)"
    r"[A-Za-z0-9_]*|close)\s*\(|\bapprox::"
)


def _test_source_files() -> list[tuple[Path, int]]:
    files: dict[Path, int] = {}
    for root in (
        REPOSITORY / "godot-bevy" / "src",
        REPOSITORY / "godot-bevy-macros" / "src",
        REPOSITORY / "godot-bevy-test" / "src",
        REPOSITORY / "godot-bevy-test-macros" / "src",
    ):
        for path in root.rglob("*.rs"):
            lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
            if path.is_relative_to(REPOSITORY / "godot-bevy" / "src" / "tests"):
                files[path] = 0
                continue
            marker = next(
                (index for index, line in enumerate(lines) if "#[cfg(test)]" in line),
                None,
            )
            if marker is not None:
                files[path] = marker
    for path in (REPOSITORY / "itest" / "rust" / "src").rglob("*.rs"):
        files[path] = 0
    return sorted(files.items())


def approximate_assertions() -> list[dict[str, str]]:
    records: list[dict[str, str]] = []
    function_start = re.compile(
        r"^(?: +)?(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+"
        r"([A-Za-z_][A-Za-z0-9_]*)"
    )
    tolerance_constant = re.compile(
        r"\bconst\s+([A-Za-z_]*(?:EPSILON|TOLERANCE)[A-Za-z_]*)\s*:[^;]+;",
        re.DOTALL,
    )
    for path, scope_start in _test_source_files():
        lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
        index = scope_start
        while index < len(lines):
            function = function_start.match(lines[index])
            if function is None:
                index += 1
                continue
            block: list[str] = []
            depth = 0
            opened = False
            cursor = index
            while cursor < len(lines):
                line = lines[cursor]
                block.append(line)
                scrubbed = re.sub(r'"(?:\\.|[^"\\])*"', "", line)
                scrubbed = scrubbed.split("//", 1)[0]
                openings = scrubbed.count("{")
                closings = scrubbed.count("}")
                opened = opened or openings > 0
                depth += openings - closings
                if opened and depth <= 0:
                    break
                cursor += 1
            require(opened and depth == 0, f"unterminated test function {path}:{index + 1}")
            source = "".join(block)
            configuration_start = index
            while configuration_start > scope_start and lines[
                configuration_start - 1
            ].lstrip().startswith("#["):
                configuration_start -= 1
            configuration = "".join(lines[configuration_start:index]) + source
            expression_lines = [
                line.strip()
                for line in source.splitlines()
                if APPROXIMATE_EXPRESSION.search(line)
            ]
            if expression_lines:
                records.append(
                    {
                        "source": path.relative_to(REPOSITORY).as_posix(),
                        "test": function.group(1),
                        "expression_fingerprint": "sha256:"
                        + hashlib.sha256("\n".join(expression_lines).encode()).hexdigest(),
                        "configuration_fingerprint": "sha256:"
                        + hashlib.sha256(configuration.encode()).hexdigest(),
                    }
                )
            index = cursor + 1

        scoped = "".join(lines[scope_start:])
        for constant in tolerance_constant.finditer(scoped):
            declaration = constant.group(0)
            digest = "sha256:" + hashlib.sha256(declaration.encode()).hexdigest()
            records.append(
                {
                    "source": path.relative_to(REPOSITORY).as_posix(),
                    "test": f"constant:{constant.group(1)}",
                    "expression_fingerprint": digest,
                    "configuration_fingerprint": digest,
                }
            )
    return sorted(records, key=lambda item: (item["source"], item["test"]))


def verify_assertions() -> None:
    ledger = load_toml(ASSERTION_LEDGER)
    require(set(ledger) == {"version", "assertions"}, "assertion ledger fields")
    require(ledger["version"] == 1, "assertion ledger version")
    expected = ledger["assertions"]
    require(isinstance(expected, list), "assertion ledger must be an array")
    actual = approximate_assertions()
    require(len(actual) == len(expected), "approximate assertion census")
    actual_by_key = {(item["source"], item["test"]): item for item in actual}
    require(len(actual_by_key) == len(actual), "duplicate approximate assertion identity")
    expected_keys = [(item.get("source"), item.get("test")) for item in expected]
    require(
        len(set(expected_keys)) == len(expected_keys),
        "duplicate assertion ledger identity",
    )
    for entry in expected:
        require(
            isinstance(entry, dict)
            and set(entry)
            == {
                "source",
                "test",
                "expression_fingerprint",
                "configuration_fingerprint",
                "units",
                "tolerance",
                "rationale",
                "sensitivity_witness",
            },
            "invalid assertion ledger entry",
        )
        key = (entry["source"], entry["test"])
        require(key in actual_by_key, f"unreviewed approximate assertion {key}")
        record = actual_by_key[key]
        require(
            record["expression_fingerprint"] == entry["expression_fingerprint"],
            f"stale approximate expression {key}",
        )
        require(
            record["configuration_fingerprint"]
            == entry["configuration_fingerprint"],
            f"stale audited test configuration {key}",
        )
        for field in ("units", "tolerance", "rationale", "sensitivity_witness"):
            require(
                isinstance(entry[field], str)
                and entry[field].strip()
                and "unresolved" not in entry[field].lower(),
                f"incomplete assertion review {key}: {field}",
            )
    write_audit_evidence(
        "assertion-audit",
        "assertions",
        ASSERTION_ARTIFACT,
        {"reviewed": len(actual), "unreviewed": 0, "stale": 0, "unwitnessed": 0},
    )
    print("PASS tolerance inventory: unreviewed=0 stale=0")
    print("PASS no-op audit: unresolved=0")
    print("PASS tolerance sensitivity: unwitnessed=0")


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
            "ordinal": 0,
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

    waived = copy.deepcopy(documents["pass"])
    waived["evidence_kind"] = "mutation-run"
    waived["checks"] = []
    waived["mutants"] = [
        {
            "id": "sha256:" + "0" * 64,
            "package": "fixture",
            "path": "src/lib.rs",
            "function": None,
            "genre": "FnValue",
            "replacement": "false",
            "source_slice_sha256": "0" * 64,
            "ordinal": 0,
            "outcome": "waived",
            "name": "fixture mutant",
            "location": {"line": 1, "column": 1},
            "log_path": None,
            "diff_path": None,
            "metadata": {
                "waiver": {"rationale": "equivalent", "reference": "fixture:waiver"}
            },
        }
    ]
    validate_qualification(waived, SCHEMA, require_complete=True)

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
        and "./itest/verify-qualification.sh faults" in ci
        and "./itest/verify-qualification.sh assertions" in ci,
        "ordinary CI lacks cheap qualification checks",
    )
    require(
        "cargo test --lib --bins --examples" in ci
        and (
            "cargo test --doc -p godot-bevy -p godot-bevy-macros "
            "-p godot-bevy-test -p godot-bevy-test-macros"
        ) in ci,
        "ordinary CI lacks published-crate doctests",
    )
    require("- 'book-tests/**'" in ci, "doctest scaffold changes bypass CI")
    checks = load_registry()
    require(
        [check.id for check in checks]
        == ["mutants", "fault-pack", "doctests", "assertions"],
        "check registry seam",
    )
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
        MutationWaiver,
        _candidate_count,
        _diff_has_production_changes,
        _missing_viable_scope,
        _source_slice,
        apply_waivers,
        diff_decision,
        load_waivers,
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
    committed_waivers = load_waivers()
    require(len(committed_waivers) == 2, "committed mutant waiver census")
    missed = run.mutants[1]
    fixture_waiver = MutationWaiver(
        stable_id=missed.stable_id,
        package=missed.package,
        path=missed.path,
        function=missed.function,
        genre=missed.genre,
        replacement=missed.replacement,
        source_slice_sha256=missed.source_slice_sha256,
        ordinal=missed.ordinal,
        rationale="fixture equivalent",
        reference="fixture:missed",
    )
    waived = apply_waivers(
        run,
        {fixture_waiver.stable_id: fixture_waiver},
        require_all=True,
    )
    require(waived.mutants[1].outcome == "waived", "mutant waiver application")
    require(
        waived.mutants[1].metadata["waiver"]["reference"] == "fixture:missed",
        "mutant waiver metadata",
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
    from qualification_mutants import EXPECTED_SCOPE, load_waivers

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
    require(len(load_waivers()) == 2, "mutant waiver manifest")
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
    "doctests": verify_doctests,
    "assertions": verify_assertions,
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
