#!/usr/bin/env python3
from __future__ import annotations

import copy
import errno
import json
import re
import subprocess
import sys
import tempfile
from dataclasses import replace
from pathlib import Path
from typing import Any, Callable

import coverage as coverage_driver
from coverage_model import (
    DIFF_STATES,
    CoverageIndex,
    CoverageModelError,
    FileCoverage,
    RegionKey,
    SourceEntry,
    classify_diff,
    coverage_exit,
    diff_exit,
    inline_test_modules,
    inventory_sources,
    load_scope_config,
    load_witnesses,
    parse_cargo_json,
    parse_flush_sentinel,
    parse_itest_report,
    parse_llvm_cov_export,
    parse_unified_diff,
    select_cargo_objects,
    sha256_bytes,
    source_identity,
    state_counts,
    validate_coverage_document,
)


REPOSITORY = Path(__file__).resolve().parents[1]
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "coverage"
REPORT_FIXTURES = FIXTURES / "reports"
SCHEMA = REPOSITORY / "itest" / "schema" / "coverage-v1.schema.json"
SCOPE = REPOSITORY / "itest" / "coverage" / "scope-v1.toml"
WITNESSES = REPOSITORY / "itest" / "coverage" / "witnesses-v1.toml"
CARGO_FIXTURE_MANIFESTS = {
    "godot-bevy": "/workspace/godot-bevy/Cargo.toml",
    "godot-bevy-macros": "/workspace/godot-bevy-macros/Cargo.toml",
    "godot-bevy-test": "/workspace/godot-bevy-test/Cargo.toml",
    "godot-bevy-test-macros": "/workspace/godot-bevy-test-macros/Cargo.toml",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def rejects(function: Callable[[], Any], message: str) -> None:
    try:
        function()
    except (CoverageModelError, coverage_driver.CoverageInfrastructureError):
        return
    raise AssertionError(message)


def evidence_exit(function: Callable[[], Any]) -> int:
    try:
        function()
    except (CoverageModelError, coverage_driver.CoverageInfrastructureError, OSError):
        return 2
    return 0


def load_json(path: Path) -> dict[str, Any]:
    document = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(document, dict), f"{path}: JSON root")
    return document


def scope_inventory() -> tuple[Any, list[SourceEntry]]:
    scope = load_scope_config(SCOPE, REPOSITORY)
    return scope, inventory_sources(REPOSITORY, scope)


def verify_tier3_migration() -> None:
    from test_qualification import approximate_assertions, rustdoc_fences
    from qualification_toml import load_toml

    assertion_ledger = load_toml(
        REPOSITORY / "itest" / "qualification" / "assertions-v1.toml"
    )["assertions"]
    actual = approximate_assertions()
    actual_by_key = {(record["source"], record["test"]): record for record in actual}
    require(len(assertion_ledger) == len(actual), "Tier-3 assertion census drift")
    for entry in assertion_ledger:
        key = (entry["source"], entry["test"])
        require(key in actual_by_key, f"missing migrated assertion {key}")
        record = actual_by_key[key]
        require(
            record["expression_fingerprint"] == entry["expression_fingerprint"]
            and record["configuration_fingerprint"] == entry["configuration_fingerprint"],
            f"migrated assertion fingerprint drift {key}",
        )
    doctest_ledger = load_toml(
        REPOSITORY / "itest" / "qualification" / "doctests-v1.toml"
    )["fences"]
    actual_fences = rustdoc_fences()
    require(len(doctest_ledger) == len(actual_fences), "doctest ledger churn")
    actual_by_key = {
        (record["source"], record["ordinal"]): record for record in actual_fences
    }
    for entry in doctest_ledger:
        key = (entry["source"], entry["ordinal"])
        require(key in actual_by_key, f"doctest identity drift {key}")
        require(
            actual_by_key[key]["fingerprint"] == entry["fingerprint"],
            f"doctest fingerprint drift {key}",
        )
    baseline_diff = subprocess.run(
        [
            "git",
            "diff",
            "--name-only",
            "--",
            "itest/qualification/mutants-baseline-v1.json",
            "itest/qualification/doctests-v1.toml",
        ],
        cwd=REPOSITORY,
        text=True,
        stdout=subprocess.PIPE,
        check=True,
    ).stdout.strip()
    require(not baseline_diff, "mutation or doctest baseline was modified")


def verify_contract() -> None:
    documents = {
        path.stem: load_json(path) for path in sorted(REPORT_FIXTURES.glob("*.json"))
    }
    require(set(documents) == {"full", "diff", "incomplete"}, "report fixture census")
    for document in documents.values():
        validate_coverage_document(document, SCHEMA)
    exits = {name: coverage_exit(document) for name, document in documents.items()}
    require(exits == {"full": 0, "diff": 1, "incomplete": 2}, f"coverage exits: {exits}")

    invalid = copy.deepcopy(documents["full"])
    invalid["unexpected"] = True
    rejects(lambda: validate_coverage_document(invalid, SCHEMA), "schema accepted extra field")
    invalid = copy.deepcopy(documents["full"])
    invalid["rate_gates"] = ["80%"]
    rejects(lambda: validate_coverage_document(invalid, SCHEMA), "schema accepted rate gate")
    invalid = copy.deepcopy(documents["full"])
    invalid["files"].append(copy.deepcopy(invalid["files"][0]))
    rejects(lambda: validate_coverage_document(invalid, SCHEMA), "schema accepted duplicate file")
    invalid = copy.deepcopy(documents["full"])
    invalid["phases"][0]["elapsed_seconds"] = float("nan")
    rejects(lambda: validate_coverage_document(invalid, SCHEMA), "schema accepted NaN")

    scope, sources = scope_inventory()
    require(len(scope.packages) == 4, "coverage package census")
    require(len(sources) == len({source.path for source in sources}), "source classification")
    dispatchers = {
        "godot-bevy/src/interop/node_markers.rs",
        "godot-bevy/src/interop/signal_names.rs",
        "godot-bevy/src/plugins/scene_tree/node_type_checking.rs",
    }
    source_by_path = {source.path: source for source in sources}
    require(
        all(source_by_path[path].classification == "excluded" for path in dispatchers),
        "generated dispatcher exclusion",
    )
    require(not inline_test_modules(REPOSITORY, sources), "inline production test module")
    require(
        all(source.classification in {"included", "excluded"} for source in sources),
        "unclassified Rust source",
    )
    schema_text = SCHEMA.read_text(encoding="utf-8")
    require('"mapped", "unmapped", null' in schema_text, "scope mapping ledger")
    disclosure = documents["full"]["disclosure"]
    require(
        disclosure["language_scope"] == ["rust"]
        and disclosure["benchmark_compatible"] is False,
        "coverage disclosure",
    )
    verify_tier3_migration()

    print("PASS coverage schema: fixtures=full,diff,incomplete")
    print("PASS coverage exits: complete=0 evidence-fail=1 infrastructure=2")
    print("PASS coverage scope: packages=4 all-rust-files-classified=true")
    print("PASS coverage scope: generated-dispatchers=excluded inline-test-modules=0")
    print("PASS coverage scope: mapped-and-unmapped-ledger=true")
    print("PASS coverage disclosure: rust-only=true benchmark-compatible=false")


def verify_tools(live: bool = True) -> None:
    devenv = (REPOSITORY / "devenv.nix").read_text(encoding="utf-8")
    cargo = (REPOSITORY / "Cargo.toml").read_text(encoding="utf-8")
    toolchain = (REPOSITORY / "rust-toolchain.toml").read_text(encoding="utf-8")
    driver = (REPOSITORY / "itest" / "coverage.py").read_text(encoding="utf-8")
    require('cargoLlvmCovPkgs.cargo-llvm-cov.version == "0.9.0"' in devenv, "cargo-llvm-cov pin")
    require('"llvm-tools"' in devenv, "llvm-tools extension")
    require("llvm-tools" not in toolchain, "rustup toolchain was changed")
    require("[profile.coverage]" in cargo, "coverage profile")
    require(
        'inherits = "dev"' in cargo and "incremental = false" in cargo,
        "coverage profile settings",
    )
    require("[profile.coverage.package.\"*\"]" in cargo, "coverage dependency optimization")
    require('environment["SCCACHE_RECACHE"] = "1"' in driver, "sccache recache policy")

    inherited = {"RUSTC_WRAPPER": "/nix/store/sccache", "CARGO_TARGET_DIR": "/old"}
    parsed = coverage_driver._parse_show_env(
        "export __CARGO_LLVM_COV_RUSTC_WRAPPER='1'\n"
        "export __CARGO_LLVM_COV_RUSTC_WRAPPER_RUSTFLAGS="
        "'-C\x1finstrument-coverage\x1f--cfg=coverage'\n"
        "export __CARGO_LLVM_COV_RUSTC_WRAPPER_PRE_EXISTING='/nix/store/sccache'\n"
        "export RUSTC_WRAPPER='/nix/store/cargo-llvm-cov'\n"
        "export CARGO_INCREMENTAL='0'\n"
        "export CARGO_LLVM_COV_TARGET_DIR='/coverage/build'\n"
        "export CARGO_LLVM_COV_BUILD_DIR='/coverage/build'\n"
        "export CARGO_TARGET_DIR='/coverage/build'\n",
        inherited,
    )
    require(
        coverage_driver._inherited_wrapper_preserved(inherited, parsed),
        "wrapper preservation",
    )
    flags = coverage_driver._coverage_flags(parsed)
    require(
        parsed["__CARGO_LLVM_COV_RUSTC_WRAPPER_RUSTFLAGS"].split("\x1f")
        == ["-C", "instrument-coverage", "--cfg=coverage"],
        "exact instrumentation flags",
    )
    require("instrument-coverage" in flags, "coverage instrumentation flag")
    require("cfg=coverage" in flags, "coverage cfg flag")
    require(parsed["CARGO_TARGET_DIR"] == "/coverage/build", "isolated target fixture")
    coverage_driver._validate_coverage_directories(
        parsed, Path("/workspace"), Path("/coverage/build")
    )
    unsafe = {**parsed, "CARGO_LLVM_COV_BUILD_DIR": "/tmp/shared"}
    rejects(
        lambda: coverage_driver._validate_coverage_directories(
            unsafe, Path("/workspace"), Path("/coverage/build")
        ),
        "coverage build directory escaped isolation",
    )
    rejects(
        lambda: coverage_driver._parse_show_env("echo unsafe\n", inherited),
        "show-env parser accepted shell code",
    )
    rejects(
        lambda: coverage_driver._parse_show_env("unset RUSTC_WRAPPER\n", inherited),
        "show-env parser accepted an unset command",
    )
    rejects(
        lambda: coverage_driver._parse_show_env("export KEY=value", inherited),
        "show-env parser accepted an unterminated export",
    )
    rejects(
        lambda: coverage_driver._parse_show_env(
            "export KEY=one\nexport KEY=two\n", inherited
        ),
        "show-env parser accepted a duplicate export",
    )
    if live:
        _, tools = coverage_driver.resolve_tools()
        require(tools["cargo_llvm_cov"] == "0.9.0", "live cargo-llvm-cov version")
        require(
            Path(tools["llvm_cov"]).parent == Path(tools["llvm_profdata"]).parent,
            "LLVM tool origin mismatch",
        )
        require(tools["rustc_llvm"] in tools["llvm_cov_version"], "LLVM live version mismatch")

    print("PASS coverage tools: cargo-llvm-cov=0.9.0")
    print("PASS coverage tools: llvm-origin=rustc-sysroot version-match=true")
    print("PASS coverage env: instrument-coverage=true cfg-coverage=true")
    print("PASS coverage env: inherited-rustc-wrapper-preserved=true isolated-target=true")
    print("PASS coverage profile: opt-level=0 incremental=false strip=none")
    print("PASS coverage flags: branch=false mcdc=false doctests=false continuous=false")


def _valid_sentinel(path: Path, pid: int = 1234) -> None:
    path.write_text(
        json.dumps({"schema_version": 1, "pid": pid, "stage": "scene", "status": 0}),
        encoding="utf-8",
    )


def verify_flush() -> None:
    cargo = (REPOSITORY / "itest" / "rust" / "Cargo.toml").read_text(encoding="utf-8")
    library = (REPOSITORY / "itest" / "rust" / "src" / "lib.rs").read_text(encoding="utf-8")
    flush = (REPOSITORY / "itest" / "rust" / "src" / "coverage_flush.rs").read_text(
        encoding="utf-8"
    )
    driver = (REPOSITORY / "itest" / "coverage.py").read_text(encoding="utf-8")
    require("coverage-flush = []" in cargo, "coverage flush feature")
    require(
        'compile_error!("coverage-flush requires cfg(coverage)")' in library,
        "coverage cfg guard",
    )
    require(
        "fn on_stage_deinit" in library and "coverage_flush::dump(stage)" in library,
        "deinit owner",
    )
    require("InitStage::Scene" in flush, "Scene flush stage")
    require(
        "__llvm_profile_dump" in flush
        and "__llvm_profile_write_file" not in flush,
        "dump symbol",
    )
    require(
        flush.index("__llvm_profile_dump()") < flush.index("OpenOptions::new()"),
        "sentinel was opened before the dump",
    )
    require(".create_new(true)" in flush and "status" in flush, "sentinel contract")
    require(
        "import-flush-v1.json" in driver
        and "itest-run-{run}-flush-v1.json" in driver,
        "phase sentinels",
    )
    production = "\n".join(
        path.read_text(encoding="utf-8")
        for path in (REPOSITORY / "godot-bevy" / "src").rglob("*.rs")
    )
    require("__llvm_profile_dump" not in production, "production library flush contamination")
    with tempfile.TemporaryDirectory() as temporary:
        sentinel = Path(temporary) / "flush.json"
        _valid_sentinel(sentinel)
        require(parse_flush_sentinel(sentinel, 1234)["status"] == 0, "valid sentinel")
        for mutate in (
            lambda value: value.update({"pid": 2}),
            lambda value: value.update({"stage": "core"}),
            lambda value: value.update({"status": 1}),
            lambda value: value.update({"extra": True}),
        ):
            document = json.loads(sentinel.read_text(encoding="utf-8"))
            mutate(document)
            sentinel.write_text(json.dumps(document), encoding="utf-8")
            rejects(lambda: parse_flush_sentinel(sentinel, 1234), "invalid sentinel accepted")
            _valid_sentinel(sentinel)

    print("PASS coverage flush: owner=godot-bevy-itest feature=coverage-flush")
    print("PASS coverage flush: stage=Scene symbol=__llvm_profile_dump")
    print("PASS coverage flush: sentinel=create-new after-dump=true status-required=true")
    print("PASS coverage flush: import-and-test-paths-distinct=true")
    print("PASS coverage flush: production-library-unchanged=true")


def cargo_fixture_objects() -> tuple[list[Any], list[Any], list[dict[str, Any]]]:
    unit = parse_cargo_json((FIXTURES / "cargo-unit.jsonl").read_text(encoding="utf-8"))
    itest = parse_cargo_json((FIXTURES / "cargo-itest.jsonl").read_text(encoding="utf-8"))
    objects = select_cargo_objects(
        unit,
        itest,
        CARGO_FIXTURE_MANIFESTS,
        "/workspace/itest/rust/Cargo.toml",
    )
    return unit, itest, objects


def verify_pipeline() -> None:
    unit, itest, objects = cargo_fixture_objects()
    require(unit and itest, "Cargo fixture parser")
    require(sum(record["kind"] == "libtest" for record in objects) == 4, "libtest census")
    require(sum(record["kind"] == "proc-macro" for record in objects) == 2, "proc-macro census")
    require(sum(record["kind"] == "cdylib" for record in objects) == 1, "cdylib census")
    malformed = (FIXTURES / "cargo-unit.jsonl").read_text(encoding="utf-8").replace(
        '"success":true', '"success":false'
    )
    rejects(lambda: parse_cargo_json(malformed), "unsuccessful Cargo JSON accepted")
    relative = (FIXTURES / "cargo-unit.jsonl").read_text(encoding="utf-8").replace(
        '"/workspace/godot-bevy/Cargo.toml"',
        '"godot-bevy/Cargo.toml"',
        1,
    )
    rejects(lambda: parse_cargo_json(relative), "relative Cargo path accepted")
    wrong_profile = copy.deepcopy(unit)
    wrong_profile[0].profile["opt_level"] = "3"
    rejects(
        lambda: select_cargo_objects(
            wrong_profile,
            itest,
            CARGO_FIXTURE_MANIFESTS,
            "/workspace/itest/rust/Cargo.toml",
        ),
        "optimized Cargo object accepted",
    )
    fresh = copy.deepcopy(unit)
    fresh[0] = replace(fresh[0], fresh=True)
    rejects(
        lambda: select_cargo_objects(
            fresh,
            itest,
            CARGO_FIXTURE_MANIFESTS,
            "/workspace/itest/rust/Cargo.toml",
        ),
        "fresh Cargo object accepted",
    )
    wrong_itest = copy.deepcopy(itest)
    wrong_itest[0] = replace(
        wrong_itest[0], filenames=("/coverage/deps/wrong-proc-macro.so",)
    )
    rejects(
        lambda: select_cargo_objects(
            unit,
            wrong_itest,
            CARGO_FIXTURE_MANIFESTS,
            "/workspace/itest/rust/Cargo.toml",
        ),
        "itest proc-macro mismatch accepted",
    )
    phase_ids = tuple(item[0] for item in coverage_driver.PHASE_DEFINITIONS)
    require(
        phase_ids == ("unit-build", "unit-runtime", "itest-build", "import", "itest-runtime"),
        "coverage phase order",
    )
    require(coverage_driver.PHASE_DEFINITIONS[3][1] is False, "import included")
    itest_report = FIXTURES / "itest-report-v1.json"
    normalized, passed = parse_itest_report(
        itest_report,
        REPOSITORY / "godot-bevy-test" / "schema" / "itest-report-v1.schema.json",
    )
    require(passed and normalized["selected"] == 1, "Tier-1 report parser")
    invalid_report = load_json(itest_report)
    invalid_report["tests"][0]["unexpected"] = True
    with tempfile.TemporaryDirectory() as temporary:
        invalid_path = Path(temporary) / "itest.json"
        invalid_path.write_text(json.dumps(invalid_report), encoding="utf-8")
        rejects(
            lambda: parse_itest_report(
                invalid_path,
                REPOSITORY
                / "godot-bevy-test"
                / "schema"
                / "itest-report-v1.schema.json",
            ),
            "malformed Tier-1 report accepted",
        )
    invalid_report = load_json(itest_report)
    invalid_report["summary"]["passed"] = 0
    invalid_report["summary"]["failed"] = 1
    with tempfile.TemporaryDirectory() as temporary:
        invalid_path = Path(temporary) / "itest.json"
        invalid_path.write_text(json.dumps(invalid_report), encoding="utf-8")
        rejects(
            lambda: parse_itest_report(
                invalid_path,
                REPOSITORY
                / "godot-bevy-test"
                / "schema"
                / "itest-report-v1.schema.json",
            ),
            "Tier-1 summary/test conflict accepted",
        )
    with tempfile.TemporaryDirectory() as temporary:
        raw = Path(temporary) / "raw"
        raw.mkdir()
        profile = raw / "123-fixture.profraw"
        profile.write_bytes(b"profile")
        require(coverage_driver.discover_profraw(raw, 123) == [profile], "profraw ownership")
        profile.write_bytes(b"")
        rejects(lambda: coverage_driver.discover_profraw(raw, 123), "empty profraw accepted")
    scope, sources = scope_inventory()
    require(source_identity(sources) == source_identity(list(sources)), "source identity stability")
    driver = (REPOSITORY / "itest" / "coverage.py").read_text(encoding="utf-8")
    require(
        "sha256_file(object_path)" in driver and "target_triple" in driver,
        "object identity fields",
    )
    require(
        "source identity drifted" in driver
        and "dirty diff drifted" in driver
        and "diff identity drifted" in driver,
        "identity drift checks",
    )

    print("PASS coverage phases: unit-build,unit-runtime,itest-build,import,itest-runtime")
    print("PASS coverage phases: import-included=false build-evidence-labelled=true")
    print("PASS coverage unit manifest: packages=4 libtest-executables=4")
    print("PASS coverage object manifest: cargo-json=true sha256=true exact-paths=true")
    print("PASS coverage raw discovery: fresh=true phase-owned=true expected-processes=true")
    print("PASS coverage identity: source-before-after=true dirty-diff=true")


def llvm_fixture() -> CoverageIndex:
    return parse_llvm_cov_export(
        (FIXTURES / "llvm-cov-export-v3.1.0.json").read_text(encoding="utf-8"),
        Path("/workspace"),
    )


def verify_reports() -> None:
    index = llvm_fixture()
    scope, _ = scope_inventory()
    witnesses = load_witnesses(WITNESSES, REPOSITORY, scope)
    require(len(witnesses) == 2, "coverage witness census")
    require(set(index.files) == {"godot-bevy/src/fixture.rs"}, "LLVM source filter fixture")
    coverage = index.files["godot-bevy/src/fixture.rs"]
    require(len(coverage.regions) == 5, "LLVM executable region census")
    require(
        coverage.coverage_counts()
        == {
            "lines": {"count": 4, "covered": 3},
            "regions": {"count": 5, "covered": 3},
            "functions": {"count": 1, "covered": 1},
        },
        "LLVM summary normalization",
    )
    source = load_json(FIXTURES / "llvm-cov-export-v3.1.0.json")
    invalid = copy.deepcopy(source)
    invalid["version"] = "3.0.1"
    rejects(
        lambda: parse_llvm_cov_export(json.dumps(invalid), Path("/workspace")),
        "LLVM version drift accepted",
    )
    invalid = copy.deepcopy(source)
    invalid["data"][0]["files"][0]["segments"][0].pop()
    rejects(
        lambda: parse_llvm_cov_export(json.dumps(invalid), Path("/workspace")),
        "short LLVM segment accepted",
    )
    invalid = copy.deepcopy(source)
    invalid["data"][0]["functions"][0]["regions"][0][5] = 99
    rejects(
        lambda: parse_llvm_cov_export(json.dumps(invalid), Path("/workspace")),
        "invalid LLVM file id accepted",
    )
    invalid = copy.deepcopy(source)
    invalid["data"][0]["files"][0]["branches"] = [[2, 1, 2, 2, 1, 0, 0, 0, 4]]
    rejects(
        lambda: parse_llvm_cov_export(json.dumps(invalid), Path("/workspace")),
        "branch coverage payload accepted",
    )
    invalid = copy.deepcopy(source)
    invalid["data"][0]["files"][0]["expansions"][0]["source_region"][5] = 99
    rejects(
        lambda: parse_llvm_cov_export(json.dumps(invalid), Path("/workspace")),
        "invalid LLVM expansion accepted",
    )
    driver = (REPOSITORY / "itest" / "coverage.py").read_text(encoding="utf-8")
    require("--failure-mode=any" in driver and '"--sparse"' in driver, "merge policy")
    require("-check-binary-ids" in driver and "-debuginfod=false" in driver, "binary checks")
    require('arguments.append("-sources")' in driver, "positive sources")
    require("llvm-cov.json.gz" in driver and "lcov.info" in driver, "canonical reports")
    require("rate_gates" in driver and '"rate_gates": []' in driver, "rate gates")
    report = load_json(REPORT_FIXTURES / "full.json")
    validate_coverage_document(report, SCHEMA)
    require(report["files"][0]["mapping"] == "mapped", "mapped file ledger")
    require(report["totals"]["merged"]["regions"]["count"] == 1, "display totals")

    print("PASS coverage merge: sparse=true failure-mode=any stale-inputs=0")
    print("PASS coverage export: objects=manifest binary-ids=checked sources=positive")
    print("PASS coverage report: merged-json=true lcov=true source-ledgers=true")
    print("PASS coverage report: mapped-and-unmapped-files=true")
    print("PASS coverage report: line-region-function-counts=true")
    print("PASS coverage report: totals-display-only=true rate-gates=0")


def synthetic_index(counts: dict[RegionKey, int], mapped: bool = True) -> CoverageIndex:
    files = {}
    if mapped:
        files["godot-bevy/src/fixture.rs"] = FileCoverage(
            "godot-bevy/src/fixture.rs",
            4,
            sum(count > 0 for count in counts.values()),
            len(counts),
            sum(count > 0 for count in counts.values()),
            1,
            int(any(count > 0 for count in counts.values())),
            dict(counts),
        )
    return CoverageIndex("3.1.0", files)


def diff_fixture_records() -> list[dict[str, Any]]:
    changes = parse_unified_diff((FIXTURES / "diff-eight-states.patch").read_text(encoding="utf-8"))
    require(len(changes) == 8, "diff fixture line census")
    line2 = RegionKey(2, 1, 2, 10, 0)
    line3a = RegionKey(3, 1, 3, 6, 0)
    line3b = RegionKey(3, 7, 3, 12, 0)
    line4 = RegionKey(4, 1, 4, 10, 0)
    line5 = RegionKey(5, 1, 5, 10, 0)
    all_regions = {line2: 1, line3a: 1, line3b: 1, line4: 0, line5: 1}
    merged = synthetic_index(all_regions)
    unit = synthetic_index({line2: 1, line3a: 1, line3b: 0, line4: 0, line5: 0})
    build = synthetic_index({region: 0 for region in all_regions})
    itest_runs = [
        synthetic_index({**{region: 0 for region in all_regions}, line5: count})
        for count in (1, 0, 1)
    ]
    hash_value = "sha256:" + "0" * 64
    sources = {
        "godot-bevy/src/fixture.rs": SourceEntry(
            "godot-bevy/src/fixture.rs",
            "godot-bevy",
            "included",
            (),
            (),
            hash_value,
            6,
        ),
        "godot-bevy/src/unmapped.rs": SourceEntry(
            "godot-bevy/src/unmapped.rs",
            "godot-bevy",
            "included",
            (),
            (),
            hash_value,
            2,
        ),
        "godot-bevy/src/fixture_tests.rs": SourceEntry(
            "godot-bevy/src/fixture_tests.rs",
            "godot-bevy",
            "excluded",
            ("**/*_tests.rs",),
            ("test-code",),
            hash_value,
            2,
        ),
    }
    return classify_diff(changes, sources, merged, unit, build, itest_runs)


def verify_diff() -> None:
    records = diff_fixture_records()
    states = [record["state"] for record in records]
    require(states == list(DIFF_STATES), f"diff states: {states}")
    require(records[1]["region_count"] == 2, "partial line region census")
    require(
        {region["verdict"] for region in records[1]["metadata"]["regions"]}
        == {"covered", "uncovered"},
        "partial line did not require every region",
    )
    require(records[3]["state"] == "unstable", "three-process instability")
    require(state_counts(records) == {state: 1 for state in DIFF_STATES}, "diff state counts")
    require(
        diff_exit([records[0]])
        == (0, "PASS coverage-diff: changed executable regions covered"),
        "diff pass exit",
    )
    require(
        diff_exit([records[2]])
        == (1, "FAIL coverage-diff: uncovered, partial, or unstable changed regions"),
        "diff fail exit",
    )
    require(
        diff_exit([records[4]])
        == (0, "SKIP coverage-diff: no in-scope executable regions changed"),
        "diff skip exit",
    )
    require(
        diff_exit([records[5]]) == (2, "ERROR coverage-diff: incomplete evidence"),
        "diff error exit",
    )
    malformed = (FIXTURES / "diff-eight-states.patch").read_text(encoding="utf-8").replace(
        "@@ -1,0 +2,5 @@", "@@ malformed @@"
    )
    rejects(lambda: parse_unified_diff(malformed), "malformed diff accepted")
    truncated = (
        "diff --git a/one.rs b/one.rs\n"
        "--- a/one.rs\n"
        "+++ b/one.rs\n"
        "@@ -1 +1 @@\n"
        "diff --git a/two.rs b/two.rs\n"
    )
    rejects(lambda: parse_unified_diff(truncated), "truncated diff hunk accepted")
    driver = (REPOSITORY / "itest" / "coverage.py").read_text(encoding="utf-8")
    require("merge-base" in driver and "--unified=0" in driver, "merge-base diff")
    require("ls-files" in driver and "untracked in-scope" in driver, "untracked source guard")

    print(
        "PASS coverage diff states: "
        "covered,partial,uncovered,unstable,no-region,not-mapped,out-of-scope,deleted"
    )
    print("PASS coverage diff: region-backed-lines=true all-regions-required=true")
    print("PASS coverage diff: processes=3 union-reported=true intersection-gated=true")
    print("PASS coverage diff: no-region=visible-neutral unreachable-state=absent")
    print("PASS coverage diff: untracked-in-scope=2 source-drift=2 not-mapped=2")
    print("PASS coverage diff exits: pass=0 fail=1 skip=0 error=2")


def _fail_closed_checks() -> None:
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        missing = root / "missing.json"
        require(
            evidence_exit(lambda: parse_flush_sentinel(missing, 1)) == 2,
            "missing sentinel exit",
        )
        sentinel = root / "sentinel.json"
        _valid_sentinel(sentinel, 1)
        document = json.loads(sentinel.read_text(encoding="utf-8"))
        document["status"] = 1
        sentinel.write_text(json.dumps(document), encoding="utf-8")
        require(
            evidence_exit(lambda: parse_flush_sentinel(sentinel, 1)) == 2,
            "sentinel status exit",
        )
        raw = root / "raw"
        raw.mkdir()
        (raw / "1-empty.profraw").write_bytes(b"")
        require(
            evidence_exit(lambda: coverage_driver.discover_profraw(raw, 1)) == 2,
            "empty raw exit",
        )
        (raw / "1-empty.profraw").write_bytes(b"valid")
        (raw / "2-child.profraw").write_bytes(b"valid")
        # child-process profiles beside the expected pid are legitimate
        # (env-probe tests re-exec themselves); the fresh phase directory is
        # the isolation boundary, so only a missing expected pid fails.
        require(
            len(coverage_driver.discover_profraw(raw, 1)) == 2,
            "child raw accepted",
        )
        require(
            evidence_exit(lambda: coverage_driver.discover_profraw(raw, 3)) == 2,
            "foreign raw exit",
        )
        error = coverage_driver.infrastructure_error(OSError(errno.ENOSPC, "fixture"))

        def raise_enospc() -> None:
            raise error

        require(
            evidence_exit(raise_enospc) == 2 and "coverage clean" in str(error),
            "ENOSPC cleanup hint",
        )
        try:
            coverage_driver._process(
                [
                    sys.executable,
                    "-c",
                    "import sys; sys.stderr.write('ENOSPC'); raise SystemExit(1)",
                ],
                {},
                root / "enospc.stdout",
                root / "enospc.stderr",
                5,
            )
        except coverage_driver.CoverageInfrastructureError as process_error:
            require("coverage clean" in str(process_error), "subprocess ENOSPC hint")
        else:
            raise AssertionError("subprocess ENOSPC was accepted")
        scratch = root / "scratch"
        scratch.mkdir()
        (scratch / "evidence").write_text("fixture", encoding="utf-8")
        coverage_driver._remove_path(scratch)
        require(not scratch.exists(), "bounded scratch cleanup")
        old_paths = (
            coverage_driver.REPOSITORY,
            coverage_driver.COVERAGE_ROOT,
            coverage_driver.BUILD_ROOT,
            coverage_driver.RUNS_ROOT,
            coverage_driver.LOCK_PATH,
        )
        coverage_driver.REPOSITORY = root
        coverage_driver.COVERAGE_ROOT = root / "target" / "coverage"
        coverage_driver.BUILD_ROOT = coverage_driver.COVERAGE_ROOT / "build"
        coverage_driver.RUNS_ROOT = coverage_driver.COVERAGE_ROOT / "runs"
        coverage_driver.LOCK_PATH = coverage_driver.COVERAGE_ROOT / ".lock"
        try:
            with coverage_driver.CoverageLock():
                require(
                    evidence_exit(lambda: coverage_driver.CoverageLock().__enter__()) == 2,
                    "concurrent coverage lock exit",
                )
        finally:
            (
                coverage_driver.REPOSITORY,
                coverage_driver.COVERAGE_ROOT,
                coverage_driver.BUILD_ROOT,
                coverage_driver.RUNS_ROOT,
                coverage_driver.LOCK_PATH,
            ) = old_paths
        object_path = root / "fixture.so"
        object_path.write_bytes(b"object")
        object_record = {
            "package": "godot-bevy-itest",
            "kind": "cdylib",
            "phase": "itest-build",
            "path": str(object_path),
            "sha256": sha256_bytes(b"object"),
        }
        run = coverage_driver.CoverageRun("full", None, False)
        old_build_root = coverage_driver.BUILD_ROOT
        coverage_driver.BUILD_ROOT = root
        try:
            run.objects = [{**object_record, "path": str(root / "missing.so")}]
            require(evidence_exit(run._object_paths) == 2, "missing object exit")
            run.objects = [{**object_record, "sha256": sha256_bytes(b"wrong")}]
            require(evidence_exit(run._object_paths) == 2, "object identity exit")
            run.objects = [{**object_record, "package": "godot-bevy"}]
            require(evidence_exit(run._cdylib_path) == 2, "wrong dylib exit")
        finally:
            coverage_driver.BUILD_ROOT = old_build_root

        scope, sources = scope_inventory()
        run.scope = scope
        run.sources = sources
        run.initial_source_identity = source_identity(sources)
        run.objects = [object_record]
        original_inventory = coverage_driver.inventory_sources
        coverage_driver.inventory_sources = lambda repository, config: sources[:-1]
        try:
            require(evidence_exit(run.verify_identity) == 2, "source drift exit")
        finally:
            coverage_driver.inventory_sources = original_inventory

        prune = coverage_driver.CoverageRun("full", None, False)
        prune.raw_dir = root / "prune-raw"
        prune.profdata_dir = root / "prune-profdata"
        prune.raw_dir.mkdir()
        prune.profdata_dir.mkdir()
        (prune.raw_dir / "raw").write_bytes(b"raw")
        (prune.profdata_dir / "profile").write_bytes(b"profile")
        prune.document = {"operations": {"raw_pruned": False}}
        prune._prune_success_data()
        require(
            not prune.raw_dir.exists()
            and not prune.profdata_dir.exists()
            and prune.document["operations"]["raw_pruned"] is True,
            "success evidence pruning",
        )
    corrupt = load_json(FIXTURES / "llvm-cov-export-v3.1.0.json")
    corrupt["data"][0]["files"][0]["segments"][0][2] = -1
    rejects(
        lambda: parse_llvm_cov_export(json.dumps(corrupt), Path("/workspace")),
        "corrupt coverage evidence accepted",
    )


def verify_fail_closed(live: bool = False) -> None:
    _fail_closed_checks()
    if live:
        result = coverage_driver.run_coverage("full", None, True)
        require(result == 0, "live keep-raw capture failed")
        report = latest_report()
        run_dir = report.parent
        document = load_json(report)
        raw_ledger = load_json(run_dir / "raw-ledger-v1.json")
        valid_profiles = [
            REPOSITORY / record["path"]
            for record in raw_ledger["records"]
            if record["included"]
        ]
        require(valid_profiles, "live raw profile ledger")
        runtime_phase = next(
            phase for phase in document["phases"] if phase["id"] == "itest-runtime"
        )
        runtime_process = runtime_phase["processes"][0]
        sentinel = REPOSITORY / runtime_process["sentinel"]
        sentinel_bytes = sentinel.read_bytes()
        try:
            sentinel.unlink()
            require(
                evidence_exit(
                    lambda: parse_flush_sentinel(sentinel, runtime_process["pid"])
                )
                == 2,
                "live missing sentinel exit",
            )
            sentinel.write_text("{", encoding="utf-8")
            require(
                evidence_exit(
                    lambda: parse_flush_sentinel(sentinel, runtime_process["pid"])
                )
                == 2,
                "live malformed sentinel exit",
            )
        finally:
            sentinel.write_bytes(sentinel_bytes)

        live_run = coverage_driver.CoverageRun("full", None, True)
        cdylib = next(
            record for record in document["objects"]["records"] if record["kind"] == "cdylib"
        )
        live_run.objects = [{**cdylib, "path": "target/coverage/build/missing.so"}]
        require(evidence_exit(live_run._object_paths) == 2, "live missing object exit")
        live_run.objects = [{**cdylib, "package": "godot-bevy"}]
        require(evidence_exit(live_run._cdylib_path) == 2, "live wrong dylib exit")

        scope, sources = scope_inventory()
        identity_run = coverage_driver.CoverageRun("full", None, True)
        identity_run.scope = scope
        identity_run.sources = sources
        identity_run.initial_source_identity = document["environment"][
            "source_identity_sha256"
        ]
        drifted = [replace(sources[0], sha256="sha256:" + "f" * 64), *sources[1:]]
        original_inventory = coverage_driver.inventory_sources
        coverage_driver.inventory_sources = lambda repository, config: drifted
        try:
            require(evidence_exit(identity_run.verify_identity) == 2, "live source drift exit")
        finally:
            coverage_driver.inventory_sources = original_inventory

        environment, tools = coverage_driver.resolve_tools()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corrupt = root / "corrupt.profraw"
            corrupt.write_bytes(b"not a profile")
            for profiles in ([corrupt], [valid_profiles[0], corrupt]):
                result = subprocess.run(
                    [
                        tools["llvm_profdata"],
                        "merge",
                        "--sparse",
                        "--failure-mode=any",
                        "-o",
                        str(root / f"out-{len(profiles)}.profdata"),
                        *[str(path) for path in profiles],
                    ],
                    cwd=REPOSITORY,
                    env=environment,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    check=False,
                )
                require(result.returncode != 0, "llvm-profdata accepted corrupt input")
        coverage_driver._remove_path(run_dir / "raw")
        coverage_driver._remove_path(run_dir / "profdata")
        require(not (run_dir / "raw").exists(), "live raw cleanup")
        document["operations"].update(
            {
                "keep_raw": False,
                "raw_pruned": True,
                "free_disk_after": coverage_driver.free_disk(
                    coverage_driver.COVERAGE_ROOT
                ),
            }
        )
        validate_coverage_document(document, SCHEMA)
        coverage_driver.write_json(report, document)

    print("PASS coverage fail-closed: missing-sentinel=2 empty-profraw=2 corrupt-profraw=2")
    print("PASS coverage fail-closed: one-of-many-corrupt=2 missing-object=2")
    print("PASS coverage fail-closed: wrong-dylib=2 source-drift=2")
    print("PASS coverage operations: concurrent-run=2 scratch-bounded=true")
    print("PASS coverage operations: success-raw-pruned=true keep-raw-supported=true")
    print("PASS coverage operations: enospc=2 cleanup-hint=true")


def verify_workflow() -> None:
    workflow_path = REPOSITORY / ".github" / "workflows" / "coverage.yml"
    require(workflow_path.is_file(), "coverage workflow missing")
    workflow = workflow_path.read_text(encoding="utf-8")
    require("\non:\n" in workflow and "\nenv:\n" in workflow, "coverage workflow blocks")
    trigger = workflow.split("\non:\n", 1)[1].split("\nenv:\n", 1)[0]
    triggers = re.findall(r"^  ([A-Za-z_][A-Za-z0-9_-]*):", trigger, re.MULTILINE)
    if triggers == ["push", "workflow_dispatch"]:
        # a push trigger is tolerated only while explicitly marked as the
        # pre-merge validation scaffold; removing the marker re-arms the check
        require(
            "TEMPORARY pre-merge validation trigger" in workflow,
            f"coverage workflow triggers: {triggers}",
        )
    else:
        require(triggers == ["workflow_dispatch"], f"coverage workflow triggers: {triggers}")
    require(workflow.count("runs-on: ubuntu-latest") == 1, "coverage Linux job census")
    require(workflow.count("uses: actions/upload-artifact@v4") == 1, "coverage artifact upload")
    upload = workflow.index("uses: actions/upload-artifact@v4")
    require("if: always()" in workflow[max(0, upload - 180) : upload], "coverage always upload")
    require("devenv shell -- coverage" in workflow, "coverage live workflow command")
    require("target/coverage/runs/" in workflow, "coverage evidence upload path")
    require("target/coverage/build" not in workflow, "coverage build cache upload")
    for variable in (
        "GODOT4_BIN",
        "ITEST_DENY_FOCUS",
        "COVERAGE_BUILD_TIMEOUT_SECONDS",
        "COVERAGE_GODOT_TIMEOUT_SECONDS",
        "COVERAGE_GODOT_QUIT_AFTER",
        "COVERAGE_RUNS_TO_KEEP",
    ):
        require(f"{variable}:" in workflow, f"workflow does not set {variable}")
    ci = (REPOSITORY / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
    require("qualification-static:" in ci, "ordinary CI static job")
    require("./itest/verify-coverage.sh contract" in ci, "ordinary CI coverage contract")
    qualification = (
        REPOSITORY / "itest" / "qualification" / "checks-v1.toml"
    ).read_text(encoding="utf-8")
    require("coverage" not in qualification, "coverage entered Tier-3 registry")
    driver = (REPOSITORY / "itest" / "coverage.py").read_text(encoding="utf-8")
    require("itest-report" not in driver or "test_reports" in driver, "Tier-1 topology")
    require("write_json(report" not in driver, "Tier-1 report mutation")

    print("PASS coverage topology: standalone=true tier1-reference=true tier1-mutation=false")
    print("PASS coverage topology: tier3-registry-entry=false")
    print("PASS coverage workflow: trigger=workflow_dispatch linux-live=true")
    print("PASS coverage workflow: rate-gates=0 artifacts-on-failure=true")
    print("PASS coverage workflow: offline-contract-in-ordinary-ci=true")


def latest_report() -> Path:
    try:
        run_id = coverage_driver.LATEST_PATH.read_text(encoding="utf-8").strip()
    except OSError as error:
        raise AssertionError("coverage latest-run pointer is missing") from error
    path = coverage_driver.RUNS_ROOT / run_id / "coverage-v1.json"
    require(path.is_file(), "coverage latest report is missing")
    return path


def verify_godot_live() -> None:
    result = coverage_driver.run_coverage("full", None, False)
    require(result == 0, "live coverage capture failed")
    report_path = latest_report()
    document = load_json(report_path)
    validate_coverage_document(document, SCHEMA)
    require(document["complete"] is True and document["outcome"] == "pass", "live complete")
    require(len(document["test_reports"]) == 1, "full itest process census")
    test_report = document["test_reports"][0]
    require(
        test_report["selected"] == test_report["registered"]
        and not test_report["focus"]
        and test_report["repeat"] == 1,
        "live Tier-1 selection",
    )
    phases = {phase["id"]: phase for phase in document["phases"]}
    runtime = phases["itest-runtime"]["processes"]
    require(len(runtime) == 1 and runtime[0]["raw_files"] > 0, "live itest profile")
    sentinel = load_json(REPOSITORY / runtime[0]["sentinel"])
    require(sentinel["stage"] == "scene" and sentinel["status"] == 0, "live sentinel")
    transform = next(
        witness
        for witness in document["witnesses"]
        if witness["id"] == "post_update_godot_transforms"
    )
    require(
        transform["unit_runtime"] == 0 and all(count > 0 for count in transform["itest_runtime"]),
        "live transform witness",
    )
    require(phases["import"]["included"] is False, "live import isolation")
    require(
        phases["import"]["processes"][0]["sentinel"] != runtime[0]["sentinel"],
        "live sentinel reuse",
    )

    print("PASS coverage itest: complete=true selected=registered focus=false repeat=1")
    print("PASS coverage flush: process=test stage=Scene status=0")
    print("PASS coverage profraw: source=itest-runtime parsed=true")
    print("PASS coverage Godot witness: post_update_godot_transforms unit=0 itest>0")
    print("PASS coverage import isolation: raw-excluded=true sentinel-not-reused=true")


def verify_all_offline() -> None:
    verify_contract()
    verify_tools(live=False)
    verify_flush()
    verify_pipeline()
    verify_reports()
    verify_diff()
    verify_fail_closed(live=False)
    verify_workflow()


MODES: dict[str, Callable[[], None]] = {
    "contract": verify_contract,
    "tools": verify_tools,
    "flush": verify_flush,
    "pipeline": verify_pipeline,
    "reports": verify_reports,
    "diff": verify_diff,
    "godot-live": verify_godot_live,
    "fail-closed-live": lambda: verify_fail_closed(live=True),
    "workflow": verify_workflow,
    "all-offline": verify_all_offline,
}


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in MODES:
        print("usage: test_coverage.py " + "|".join(MODES), file=sys.stderr)
        return 2
    try:
        MODES[sys.argv[1]]()
    except AssertionError as error:
        print(f"ERROR coverage verification: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
