#!/usr/bin/env python3
from __future__ import annotations

import fnmatch
import hashlib
import json
import math
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable

try:
    import tomllib

    def _toml_loads(text: str) -> dict[str, Any]:
        return tomllib.loads(text)

except ImportError:
    from qualification_toml import loads_toml as _toml_loads


LLVM_EXPORT_TYPE = "llvm.coverage.json.export"
LLVM_EXPORT_VERSION = "3.1.0"
COVERAGE_SOURCES = ("unit-runtime", "test-build", "itest-runtime")
DIFF_STATES = (
    "covered",
    "partial",
    "uncovered",
    "unstable",
    "no-region",
    "not-mapped",
    "out-of-scope",
    "deleted",
)
INLINE_TEST_MODULE = re.compile(
    r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*"
    r"(?:#\s*\[[^\]]+\]\s*)*mod\s+[A-Za-z_][A-Za-z0-9_]*\s*\{",
    re.MULTILINE,
)


class CoverageModelError(ValueError):
    pass


def sha256_bytes(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    try:
        return sha256_bytes(path.read_bytes())
    except OSError as error:
        raise CoverageModelError(f"could not hash {path}: {error}") from error


def canonical_json_bytes(value: Any) -> bytes:
    return (
        json.dumps(
            value,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        )
        + "\n"
    ).encode()


def relative_path(path: Path, repository: Path) -> str:
    try:
        return path.resolve().relative_to(repository.resolve()).as_posix()
    except ValueError as error:
        raise CoverageModelError(f"path escapes repository: {path}") from error


def _safe_repository_path(repository: Path, value: str, label: str) -> Path:
    path = Path(value)
    if path.is_absolute() or ".." in path.parts:
        raise CoverageModelError(f"{label} must be repository-relative: {value!r}")
    resolved = (repository / path).resolve()
    try:
        resolved.relative_to(repository.resolve())
    except ValueError as error:
        raise CoverageModelError(f"{label} escapes the repository: {value!r}") from error
    return resolved


@dataclass(frozen=True)
class PackageScope:
    name: str
    root: str


@dataclass(frozen=True)
class Exclusion:
    pattern: str
    category: str


@dataclass(frozen=True)
class ScopeConfig:
    version: int
    packages: tuple[PackageScope, ...]
    exclusions: tuple[Exclusion, ...]

    def package_for(self, path: str) -> PackageScope | None:
        for package in self.packages:
            if path == package.root or path.startswith(package.root + "/"):
                return package
        return None

    def matching_exclusions(self, path: str) -> tuple[Exclusion, ...]:
        return tuple(
            exclusion
            for exclusion in self.exclusions
            if fnmatch.fnmatchcase(path, exclusion.pattern)
        )


@dataclass(frozen=True)
class SourceEntry:
    path: str
    package: str
    classification: str
    exclusion_patterns: tuple[str, ...]
    exclusion_categories: tuple[str, ...]
    sha256: str
    source_lines: int

    def to_scope_record(self, mapping: str | None = None) -> dict[str, Any]:
        return {
            "path": self.path,
            "package": self.package,
            "classification": self.classification,
            "exclusion_patterns": list(self.exclusion_patterns),
            "exclusion_categories": list(self.exclusion_categories),
            "sha256": self.sha256,
            "source_lines": self.source_lines,
            "mapping": mapping,
        }


@dataclass(frozen=True)
class WitnessConfig:
    id: str
    source: str
    function: str
    line_contains: str
    test: str
    unit_runtime: str
    itest_runtime: str


def _load_toml(path: Path) -> dict[str, Any]:
    try:
        document = _toml_loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise CoverageModelError(f"could not load {path}: {error}") from error
    if not isinstance(document, dict):
        raise CoverageModelError(f"{path}: TOML root must be a table")
    return document


def load_scope_config(path: Path, repository: Path) -> ScopeConfig:
    document = _load_toml(path)
    if set(document) != {"version", "packages", "exclusions"}:
        raise CoverageModelError("coverage scope has invalid top-level fields")
    if document["version"] != 1:
        raise CoverageModelError("coverage scope version must be 1")
    raw_packages = document["packages"]
    raw_exclusions = document["exclusions"]
    if not isinstance(raw_packages, list) or not raw_packages:
        raise CoverageModelError("coverage scope packages must be a nonempty array")
    if not isinstance(raw_exclusions, list):
        raise CoverageModelError("coverage scope exclusions must be an array")

    packages: list[PackageScope] = []
    for index, raw in enumerate(raw_packages):
        if not isinstance(raw, dict) or set(raw) != {"name", "root"}:
            raise CoverageModelError(f"packages[{index}] has invalid fields")
        name, root = raw["name"], raw["root"]
        if not isinstance(name, str) or not name:
            raise CoverageModelError(f"packages[{index}].name must be nonempty")
        if not isinstance(root, str) or not root:
            raise CoverageModelError(f"packages[{index}].root must be nonempty")
        root_path = _safe_repository_path(repository, root, f"packages[{index}].root")
        if not root_path.is_dir():
            raise CoverageModelError(f"coverage package root is missing: {root}")
        packages.append(PackageScope(name, Path(root).as_posix().rstrip("/")))

    exclusions: list[Exclusion] = []
    for index, raw in enumerate(raw_exclusions):
        if not isinstance(raw, dict) or set(raw) != {"pattern", "category"}:
            raise CoverageModelError(f"exclusions[{index}] has invalid fields")
        pattern, category = raw["pattern"], raw["category"]
        if not isinstance(pattern, str) or not pattern or Path(pattern).is_absolute():
            raise CoverageModelError(f"exclusions[{index}].pattern is invalid")
        if ".." in Path(pattern).parts:
            raise CoverageModelError(f"exclusions[{index}].pattern escapes the repository")
        if not isinstance(category, str) or not category:
            raise CoverageModelError(f"exclusions[{index}].category must be nonempty")
        exclusions.append(Exclusion(pattern, category))

    names = [package.name for package in packages]
    roots = [package.root for package in packages]
    rules = [(exclusion.pattern, exclusion.category) for exclusion in exclusions]
    if len(names) != len(set(names)) or len(roots) != len(set(roots)):
        raise CoverageModelError("coverage scope contains duplicate packages")
    if len(rules) != len(set(rules)):
        raise CoverageModelError("coverage scope contains duplicate exclusions")
    for left in roots:
        for right in roots:
            if left != right and left.startswith(right + "/"):
                raise CoverageModelError("coverage package roots must not overlap")
    return ScopeConfig(1, tuple(packages), tuple(exclusions))


def inventory_sources(repository: Path, scope: ScopeConfig) -> list[SourceEntry]:
    entries: list[SourceEntry] = []
    seen: set[str] = set()
    for package in scope.packages:
        root = _safe_repository_path(repository, package.root, "package root")
        for path in sorted(root.rglob("*.rs")):
            if not path.is_file():
                continue
            relative = relative_path(path, repository)
            if relative in seen:
                raise CoverageModelError(f"source appears in multiple package roots: {relative}")
            seen.add(relative)
            matches = scope.matching_exclusions(relative)
            data = path.read_bytes()
            entries.append(
                SourceEntry(
                    relative,
                    package.name,
                    "excluded" if matches else "included",
                    tuple(match.pattern for match in matches),
                    tuple(sorted({match.category for match in matches})),
                    sha256_bytes(data),
                    len(data.decode("utf-8").splitlines()),
                )
            )
    if not entries:
        raise CoverageModelError("coverage scope inventory is empty")
    return sorted(entries, key=lambda entry: entry.path)


def source_identity(entries: Iterable[SourceEntry]) -> str:
    records = [
        {"path": entry.path, "sha256": entry.sha256}
        for entry in entries
        if entry.classification == "included"
    ]
    return sha256_bytes(canonical_json_bytes(records))


def inline_test_modules(repository: Path, entries: Iterable[SourceEntry]) -> list[str]:
    found: list[str] = []
    for entry in entries:
        if entry.classification != "included":
            continue
        text = (repository / entry.path).read_text(encoding="utf-8")
        if INLINE_TEST_MODULE.search(text):
            found.append(entry.path)
    return found


def load_witnesses(path: Path, repository: Path, scope: ScopeConfig) -> list[WitnessConfig]:
    document = _load_toml(path)
    if set(document) != {"version", "witnesses"} or document["version"] != 1:
        raise CoverageModelError("coverage witness ledger has invalid fields or version")
    raw_witnesses = document["witnesses"]
    if not isinstance(raw_witnesses, list) or not raw_witnesses:
        raise CoverageModelError("coverage witness ledger is empty")
    witnesses: list[WitnessConfig] = []
    expected = {
        "id",
        "source",
        "function",
        "line_contains",
        "test",
        "unit_runtime",
        "itest_runtime",
    }
    for index, raw in enumerate(raw_witnesses):
        if not isinstance(raw, dict) or set(raw) != expected:
            raise CoverageModelError(f"witnesses[{index}] has invalid fields")
        if not all(isinstance(raw[key], str) and raw[key] for key in expected):
            raise CoverageModelError(f"witnesses[{index}] contains an empty value")
        if raw["unit_runtime"] not in {"zero", "positive"}:
            raise CoverageModelError(f"witnesses[{index}].unit_runtime is invalid")
        if raw["itest_runtime"] not in {"zero", "positive"}:
            raise CoverageModelError(f"witnesses[{index}].itest_runtime is invalid")
        source = raw["source"]
        if scope.package_for(source) is None or scope.matching_exclusions(source):
            raise CoverageModelError(f"witness source is outside coverage scope: {source}")
        source_path = _safe_repository_path(repository, source, "witness source")
        if not source_path.is_file():
            raise CoverageModelError(f"witness source is missing: {source}")
        source_text = source_path.read_text(encoding="utf-8")
        matching_lines = [
            line
            for line in source_text.splitlines()
            if raw["line_contains"] in line
        ]
        if len(matching_lines) != 1:
            raise CoverageModelError(
                f"witness marker must occur exactly once in {source}: {raw['line_contains']!r}"
            )
        function_matches = list(
            re.finditer(rf"\bfn\s+{re.escape(raw['function'])}\s*(?:<|\()", source_text)
        )
        marker_offset = source_text.index(raw["line_contains"])
        if len(function_matches) != 1 or function_matches[0].start() > marker_offset:
            raise CoverageModelError(
                f"witness marker is not in the named function: {raw['function']}"
            )
        witnesses.append(WitnessConfig(**raw))
    ids = [witness.id for witness in witnesses]
    if len(ids) != len(set(ids)):
        raise CoverageModelError("coverage witness ledger contains duplicate ids")
    return witnesses


@dataclass(frozen=True)
class CargoArtifact:
    package_id: str
    manifest_path: str
    target_name: str
    target_kind: tuple[str, ...]
    crate_types: tuple[str, ...]
    profile: dict[str, Any]
    features: tuple[str, ...]
    filenames: tuple[str, ...]
    executable: str | None
    fresh: bool


def _string_array(value: Any, label: str) -> tuple[str, ...]:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise CoverageModelError(f"{label} must be a string array")
    return tuple(value)


def parse_cargo_json(text: str) -> list[CargoArtifact]:
    artifacts: list[CargoArtifact] = []
    build_finished = False
    for line_number, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            message = json.loads(line)
        except json.JSONDecodeError as error:
            raise CoverageModelError(f"Cargo JSON line {line_number} is malformed") from error
        if not isinstance(message, dict) or not isinstance(message.get("reason"), str):
            raise CoverageModelError(f"Cargo JSON line {line_number} has no reason")
        if message["reason"] == "build-finished":
            if message.get("success") is not True:
                raise CoverageModelError("Cargo JSON reports an unsuccessful build")
            build_finished = True
            continue
        if message["reason"] != "compiler-artifact":
            continue
        target = message.get("target")
        profile = message.get("profile")
        required = ("package_id", "manifest_path", "features", "filenames", "fresh")
        if not isinstance(target, dict) or not isinstance(profile, dict):
            raise CoverageModelError(f"Cargo artifact line {line_number} is incomplete")
        if not all(key in message for key in required):
            raise CoverageModelError(f"Cargo artifact line {line_number} is incomplete")
        if not all(key in target for key in ("name", "kind", "crate_types")):
            raise CoverageModelError(f"Cargo target line {line_number} is incomplete")
        executable = message.get("executable")
        if executable is not None and not isinstance(executable, str):
            raise CoverageModelError(f"Cargo executable line {line_number} is invalid")
        if not isinstance(message["package_id"], str) or not isinstance(
            message["manifest_path"], str
        ):
            raise CoverageModelError(f"Cargo package line {line_number} is invalid")
        if not Path(message["manifest_path"]).is_absolute():
            raise CoverageModelError(
                f"Cargo manifest line {line_number} is not absolute"
            )
        if not isinstance(target["name"], str) or not isinstance(message["fresh"], bool):
            raise CoverageModelError(f"Cargo artifact line {line_number} is invalid")
        for key in (
            "opt_level",
            "debuginfo",
            "debug_assertions",
            "overflow_checks",
            "test",
        ):
            if key not in profile:
                raise CoverageModelError(f"Cargo profile lacks {key!r}")
        filenames = _string_array(message["filenames"], "Cargo filenames")
        if any(not Path(filename).is_absolute() for filename in filenames):
            raise CoverageModelError(
                f"Cargo filename line {line_number} is not absolute"
            )
        if executable is not None and not Path(executable).is_absolute():
            raise CoverageModelError(
                f"Cargo executable line {line_number} is not absolute"
            )
        artifacts.append(
            CargoArtifact(
                message["package_id"],
                message["manifest_path"],
                target["name"],
                _string_array(target["kind"], "Cargo target.kind"),
                _string_array(target["crate_types"], "Cargo target.crate_types"),
                {key: profile[key] for key in (
                    "opt_level",
                    "debuginfo",
                    "debug_assertions",
                    "overflow_checks",
                    "test",
                )},
                _string_array(message["features"], "Cargo features"),
                filenames,
                executable,
                message["fresh"],
            )
        )
    if not build_finished:
        raise CoverageModelError("Cargo JSON lacks a successful build-finished message")
    return artifacts


def _manifest_key(path: str) -> str:
    return Path(path).resolve().as_posix()


def _dynamic_library(paths: Iterable[str]) -> str | None:
    candidates = sorted(
        path for path in paths if Path(path).suffix.lower() in {".so", ".dylib", ".dll"}
    )
    if len(candidates) > 1:
        raise CoverageModelError(f"Cargo artifact has multiple dynamic libraries: {candidates}")
    return candidates[0] if candidates else None


def select_cargo_objects(
    unit_artifacts: Iterable[CargoArtifact],
    itest_artifacts: Iterable[CargoArtifact],
    package_manifests: dict[str, str],
    itest_manifest: str,
) -> list[dict[str, Any]]:
    unit_artifacts = list(unit_artifacts)
    itest_artifacts = list(itest_artifacts)
    by_manifest = {_manifest_key(path): name for name, path in package_manifests.items()}
    selected: list[dict[str, Any]] = []
    libtests: dict[str, CargoArtifact] = {}
    for artifact in unit_artifacts:
        package = by_manifest.get(_manifest_key(artifact.manifest_path))
        if package is None:
            continue
        if artifact.profile["test"] is True and artifact.executable is not None:
            if package in libtests:
                raise CoverageModelError(f"multiple libtest executables for {package}")
            libtests[package] = artifact
            selected.append(
                _cargo_object_record(
                    package, artifact, "libtest", "unit-build", artifact.executable
                )
            )
        if "proc-macro" in artifact.target_kind and artifact.profile["test"] is False:
            library = _dynamic_library(artifact.filenames)
            if library is not None:
                selected.append(
                    _cargo_object_record(package, artifact, "proc-macro", "unit-build", library)
                )
    missing = sorted(set(package_manifests) - set(libtests))
    if missing:
        raise CoverageModelError(f"missing libtest executables: {', '.join(missing)}")

    itest_key = _manifest_key(itest_manifest)
    cdylibs: list[tuple[CargoArtifact, str]] = []
    for artifact in itest_artifacts:
        if _manifest_key(artifact.manifest_path) != itest_key:
            continue
        if "cdylib" in artifact.crate_types and artifact.profile["test"] is False:
            library = _dynamic_library(artifact.filenames)
            if library is not None:
                cdylibs.append((artifact, library))
    if len(cdylibs) != 1:
        raise CoverageModelError(f"expected one itest cdylib, found {len(cdylibs)}")
    artifact, library = cdylibs[0]
    selected.append(
        _cargo_object_record("godot-bevy-itest", artifact, "cdylib", "itest-build", library)
    )

    # Cargo may unify features differently between the unit and itest graphs,
    # producing a second legitimate copy of an in-scope proc-macro dylib. Both
    # carry coverage maps for the same sources, so manifest every Cargo-reported
    # copy instead of demanding a single identity.
    unit_proc_macros = {
        record["path"] for record in selected if record["kind"] == "proc-macro"
    }
    for artifact in itest_artifacts:
        if (
            _manifest_key(artifact.manifest_path) in by_manifest
            and "proc-macro" in artifact.target_kind
            and artifact.profile["test"] is False
        ):
            path = _dynamic_library(artifact.filenames)
            if path is not None and path not in unit_proc_macros:
                selected.append(
                    _cargo_object_record(
                        artifact.target_name.replace("_", "-"),
                        artifact,
                        "proc-macro",
                        "itest-build",
                        path,
                    )
                )

    unique: dict[str, dict[str, Any]] = {}
    for record in selected:
        existing = unique.get(record["path"])
        if existing is not None and existing != record:
            raise CoverageModelError(f"conflicting Cargo object records: {record['path']}")
        unique[record["path"]] = record
    records = sorted(
        unique.values(),
        key=lambda record: (record["kind"], record["package"], record["path"]),
    )
    if sum(record["kind"] == "libtest" for record in records) != len(package_manifests):
        raise CoverageModelError("Cargo object manifest has an invalid libtest census")
    return records


def _cargo_object_record(
    package: str,
    artifact: CargoArtifact,
    kind: str,
    phase: str,
    path: str,
) -> dict[str, Any]:
    if artifact.fresh:
        raise CoverageModelError(f"coverage object {artifact.target_name} was not rebuilt")
    if artifact.profile["opt_level"] != "0":
        raise CoverageModelError(
            f"coverage object {artifact.target_name} has opt-level {artifact.profile['opt_level']}"
        )
    return {
        "package": package,
        "target": artifact.target_name,
        "kind": kind,
        "phase": phase,
        "path": path,
        "profile": {"name": "coverage", **artifact.profile},
        "features": sorted(set(artifact.features)),
    }


@dataclass(frozen=True, order=True)
class RegionKey:
    start_line: int
    start_column: int
    end_line: int
    end_column: int
    kind: int

    def intersects_line(self, line: int) -> bool:
        if line < self.start_line or line > self.end_line:
            return False
        if self.start_line == self.end_line:
            return self.start_column < self.end_column and line == self.start_line
        if line == self.end_line and self.end_column == 1:
            return False
        return True


@dataclass
class FileCoverage:
    path: str
    lines_count: int
    lines_covered: int
    regions_count: int
    regions_covered: int
    functions_count: int
    functions_covered: int
    regions: dict[RegionKey, int] = field(default_factory=dict)

    def coverage_counts(self) -> dict[str, dict[str, int]]:
        return {
            "lines": {"count": self.lines_count, "covered": self.lines_covered},
            "regions": {"count": self.regions_count, "covered": self.regions_covered},
            "functions": {"count": self.functions_count, "covered": self.functions_covered},
        }


@dataclass
class CoverageIndex:
    version: str
    files: dict[str, FileCoverage]

    def count(self, path: str, region: RegionKey) -> int:
        coverage = self.files.get(path)
        if coverage is None:
            return 0
        return coverage.regions.get(region, 0)


def _json_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise CoverageModelError(f"{label} must be an object")
    return value


def _json_array(value: Any, label: str) -> list[Any]:
    if not isinstance(value, list):
        raise CoverageModelError(f"{label} must be an array")
    return value


def _nonnegative_integer(value: Any, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        raise CoverageModelError(f"{label} must be a nonnegative integer")
    return value


def _positive_integer(value: Any, label: str) -> int:
    value = _nonnegative_integer(value, label)
    if value == 0:
        raise CoverageModelError(f"{label} must be positive")
    return value


def _normalize_llvm_filename(filename: Any, repository: Path) -> str:
    if not isinstance(filename, str) or not filename:
        raise CoverageModelError("LLVM filename must be nonempty")
    path = Path(filename)
    if not path.is_absolute():
        path = repository / path
    return relative_path(path, repository)


def _optional_llvm_filename(filename: Any, repository: Path) -> str | None:
    if not isinstance(filename, str) or not filename:
        raise CoverageModelError("LLVM filename must be nonempty")
    path = Path(filename)
    if not path.is_absolute():
        path = repository / path
    try:
        return path.resolve().relative_to(repository.resolve()).as_posix()
    except ValueError:
        return None


def _summary_pair(value: Any, label: str, detailed: bool) -> tuple[int, int]:
    record = _json_object(value, label)
    expected = {"count", "covered", "percent"}
    if detailed:
        expected.add("notcovered")
    if set(record) != expected:
        raise CoverageModelError(f"{label} has invalid fields")
    count = _nonnegative_integer(record["count"], f"{label}.count")
    covered = _nonnegative_integer(record["covered"], f"{label}.covered")
    if covered > count:
        raise CoverageModelError(f"{label} covered exceeds count")
    if detailed and record["notcovered"] != count - covered:
        raise CoverageModelError(f"{label}.notcovered conflicts with count")
    percent = record["percent"]
    if (
        not isinstance(percent, (int, float))
        or isinstance(percent, bool)
        or not math.isfinite(percent)
        or not 0 <= percent <= 100
    ):
        raise CoverageModelError(f"{label}.percent is invalid")
    return count, covered


def _coverage_summary(value: Any, label: str) -> tuple[int, int, int, int, int, int]:
    summary = _json_object(value, label)
    expected = {"lines", "functions", "instantiations", "regions", "branches", "mcdc"}
    if set(summary) != expected:
        raise CoverageModelError(f"{label} has invalid fields")
    lines = _summary_pair(summary["lines"], f"{label}.lines", False)
    regions = _summary_pair(summary["regions"], f"{label}.regions", True)
    functions = _summary_pair(summary["functions"], f"{label}.functions", False)
    _summary_pair(summary["instantiations"], f"{label}.instantiations", False)
    for name in ("branches", "mcdc"):
        count, covered = _summary_pair(summary[name], f"{label}.{name}", True)
        if count != 0 or covered != 0:
            raise CoverageModelError(f"{label}.{name} coverage is forbidden")
    return (*lines, *regions, *functions)


def _parse_segment(value: Any, label: str) -> None:
    segment = _json_array(value, label)
    if len(segment) != 6:
        raise CoverageModelError(f"{label} must contain six fields")
    _positive_integer(segment[0], f"{label}[0]")
    _positive_integer(segment[1], f"{label}[1]")
    _nonnegative_integer(segment[2], f"{label}[2]")
    if not all(isinstance(segment[index], bool) for index in (3, 4, 5)):
        raise CoverageModelError(f"{label} flags must be boolean")


def _parse_region(value: Any, label: str) -> tuple[RegionKey, int, int]:
    region = _json_array(value, label)
    if len(region) != 8:
        raise CoverageModelError(f"{label} must contain eight fields")
    start_line = _positive_integer(region[0], f"{label}[0]")
    start_column = _positive_integer(region[1], f"{label}[1]")
    end_line = _positive_integer(region[2], f"{label}[2]")
    end_column = _positive_integer(region[3], f"{label}[3]")
    count = _nonnegative_integer(region[4], f"{label}[4]")
    file_id = _nonnegative_integer(region[5], f"{label}[5]")
    _nonnegative_integer(region[6], f"{label}[6]")
    kind = _nonnegative_integer(region[7], f"{label}[7]")
    if (end_line, end_column) < (start_line, start_column):
        raise CoverageModelError(f"{label} has reversed coordinates")
    return RegionKey(start_line, start_column, end_line, end_column, kind), count, file_id


def _require_empty_array(record: dict[str, Any], field: str, label: str) -> None:
    values = _json_array(record.get(field), f"{label}.{field}")
    if values:
        raise CoverageModelError(f"{label}.{field} must be empty")


def _parse_expansion(value: Any, label: str) -> None:
    expansion = _json_object(value, label)
    if set(expansion) != {"filenames", "source_region", "target_regions", "branches"}:
        raise CoverageModelError(f"{label} has invalid fields")
    filenames = _string_array(expansion["filenames"], f"{label}.filenames")
    if not filenames:
        raise CoverageModelError(f"{label}.filenames must not be empty")
    _, _, source_file_id = _parse_region(expansion["source_region"], f"{label}.source_region")
    if source_file_id >= len(filenames):
        raise CoverageModelError(f"{label}.source_region has an invalid file id")
    for index, raw_region in enumerate(
        _json_array(expansion["target_regions"], f"{label}.target_regions")
    ):
        _, _, file_id = _parse_region(raw_region, f"{label}.target_regions[{index}]")
        if file_id >= len(filenames):
            raise CoverageModelError(f"{label}.target_regions[{index}] has an invalid file id")
    if _json_array(expansion["branches"], f"{label}.branches"):
        raise CoverageModelError(f"{label}.branches must be empty")


def parse_llvm_cov_export(text: str, repository: Path) -> CoverageIndex:
    try:
        document = json.loads(text)
    except json.JSONDecodeError as error:
        raise CoverageModelError("LLVM coverage export is malformed JSON") from error
    root = _json_object(document, "LLVM export")
    if set(root) != {"type", "version", "data"}:
        raise CoverageModelError("LLVM coverage export has invalid top-level fields")
    if root.get("type") != LLVM_EXPORT_TYPE:
        raise CoverageModelError(f"unexpected LLVM export type: {root.get('type')!r}")
    if root.get("version") != LLVM_EXPORT_VERSION:
        raise CoverageModelError(
            f"unsupported LLVM export version {root.get('version')!r}; "
            f"expected {LLVM_EXPORT_VERSION}"
        )
    data = _json_array(root.get("data"), "LLVM export.data")
    if len(data) != 1:
        raise CoverageModelError("LLVM coverage export must contain exactly one data object")
    export = _json_object(data[0], "LLVM export.data[0]")
    if set(export) != {"files", "functions", "totals"}:
        raise CoverageModelError("LLVM export.data[0] has invalid fields")
    raw_files = _json_array(export.get("files"), "LLVM export files")
    raw_functions = _json_array(export.get("functions"), "LLVM export functions")
    _coverage_summary(export.get("totals"), "LLVM export totals")

    files: dict[str, FileCoverage] = {}
    for index, raw_file in enumerate(raw_files):
        record = _json_object(raw_file, f"LLVM files[{index}]")
        if set(record) != {
            "filename",
            "segments",
            "branches",
            "mcdc_records",
            "expansions",
            "summary",
        }:
            raise CoverageModelError(f"LLVM files[{index}] has invalid fields")
        path = _normalize_llvm_filename(record.get("filename"), repository)
        if path in files:
            raise CoverageModelError(f"duplicate LLVM file mapping: {path}")
        for segment_index, segment in enumerate(
            _json_array(record.get("segments"), f"LLVM files[{index}].segments")
        ):
            _parse_segment(segment, f"LLVM files[{index}].segments[{segment_index}]")
        _require_empty_array(record, "branches", f"LLVM files[{index}]")
        _require_empty_array(record, "mcdc_records", f"LLVM files[{index}]")
        for expansion_index, expansion in enumerate(
            _json_array(record.get("expansions"), f"LLVM files[{index}].expansions")
        ):
            _parse_expansion(
                expansion,
                f"LLVM files[{index}].expansions[{expansion_index}]",
            )
        lines, lines_covered, regions, regions_covered, functions, functions_covered = (
            _coverage_summary(record.get("summary"), f"LLVM files[{index}].summary")
        )
        files[path] = FileCoverage(
            path,
            lines,
            lines_covered,
            regions,
            regions_covered,
            functions,
            functions_covered,
        )

    for function_index, raw_function in enumerate(raw_functions):
        function = _json_object(raw_function, f"LLVM functions[{function_index}]")
        if set(function) != {
            "name",
            "count",
            "filenames",
            "regions",
            "branches",
            "mcdc_records",
        }:
            raise CoverageModelError(f"LLVM functions[{function_index}] has invalid fields")
        if not isinstance(function["name"], str) or not function["name"]:
            raise CoverageModelError(f"LLVM functions[{function_index}].name is invalid")
        filenames = [
            _optional_llvm_filename(filename, repository)
            for filename in _json_array(
                function.get("filenames"), f"LLVM functions[{function_index}].filenames"
            )
        ]
        if not filenames:
            raise CoverageModelError(f"LLVM functions[{function_index}] has no filenames")
        _nonnegative_integer(function.get("count"), f"LLVM functions[{function_index}].count")
        _require_empty_array(function, "branches", f"LLVM functions[{function_index}]")
        _require_empty_array(function, "mcdc_records", f"LLVM functions[{function_index}]")
        for region_index, raw_region in enumerate(
            _json_array(function.get("regions"), f"LLVM functions[{function_index}].regions")
        ):
            region, count, file_id = _parse_region(
                raw_region, f"LLVM functions[{function_index}].regions[{region_index}]"
            )
            if file_id >= len(filenames):
                raise CoverageModelError(
                    f"LLVM functions[{function_index}] region has invalid file id {file_id}"
                )
            if region.kind in {2, 3}:
                continue
            path = filenames[file_id]
            if path not in files:
                continue
            assert path is not None
            files[path].regions[region] = max(files[path].regions.get(region, 0), count)
    return CoverageIndex(LLVM_EXPORT_VERSION, files)


def validate_mapping_identity(
    merged: CoverageIndex,
    others: Iterable[CoverageIndex],
    included_paths: Iterable[str],
) -> None:
    for path in included_paths:
        merged_file = merged.files.get(path)
        expected = set(merged_file.regions) if merged_file is not None else set()
        for index, coverage in enumerate(others):
            other_file = coverage.files.get(path)
            actual = set(other_file.regions) if other_file is not None else set()
            if (merged_file is None) != (other_file is None) or actual != expected:
                raise CoverageModelError(
                    f"coverage mapping identity differs for {path} in export {index + 1}"
                )


def empty_coverage_counts() -> dict[str, dict[str, int]]:
    return {
        name: {"count": 0, "covered": 0}
        for name in ("lines", "regions", "functions")
    }


def sum_coverage_counts(values: Iterable[dict[str, dict[str, int]]]) -> dict[str, dict[str, int]]:
    total = empty_coverage_counts()
    for value in values:
        for name in total:
            total[name]["count"] += value[name]["count"]
            total[name]["covered"] += value[name]["covered"]
    return total


@dataclass(frozen=True)
class DiffChange:
    path: str
    old_line: int | None
    new_line: int | None
    kind: str


HUNK = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")


def _diff_path(header: str, prefix: str) -> str | None:
    value = header[len(prefix) :]
    if value == "/dev/null":
        return None
    if value.startswith("a/") or value.startswith("b/"):
        value = value[2:]
    return value


def parse_unified_diff(text: str) -> list[DiffChange]:
    changes: list[DiffChange] = []
    old_path: str | None = None
    new_path: str | None = None
    old_line: int | None = None
    new_line: int | None = None
    old_remaining = 0
    new_remaining = 0
    for line_number, line in enumerate(text.splitlines(), 1):
        if line.startswith("diff --git "):
            if old_remaining != 0 or new_remaining != 0:
                raise CoverageModelError(
                    f"diff hunk ended early before line {line_number}"
                )
            old_path = None
            new_path = None
            old_line = None
            new_line = None
            old_remaining = 0
            new_remaining = 0
            continue
        if line.startswith("@@ "):
            if old_remaining != 0 or new_remaining != 0:
                raise CoverageModelError(
                    f"diff hunk ended early before line {line_number}"
                )
            match = HUNK.match(line)
            if match is None or (old_path is None and new_path is None):
                raise CoverageModelError(f"malformed diff hunk at line {line_number}")
            old_line = int(match.group(1))
            new_line = int(match.group(3))
            old_remaining = int(match.group(2) or 1)
            new_remaining = int(match.group(4) or 1)
            continue
        if old_line is not None and new_line is not None:
            if line.startswith("\\ No newline at end of file"):
                continue
            if line.startswith("-"):
                if old_path is None or old_remaining == 0:
                    raise CoverageModelError(
                        f"diff deletion has no old line at line {line_number}"
                    )
                changes.append(DiffChange(old_path, old_line, None, "deleted"))
                old_line += 1
                old_remaining -= 1
            elif line.startswith("+"):
                if new_path is None or new_remaining == 0:
                    raise CoverageModelError(
                        f"diff addition has no new line at line {line_number}"
                    )
                changes.append(DiffChange(new_path, None, new_line, "added"))
                new_line += 1
                new_remaining -= 1
            elif line.startswith(" "):
                if old_remaining == 0 or new_remaining == 0:
                    raise CoverageModelError(f"unexpected diff context at line {line_number}")
                old_line += 1
                new_line += 1
                old_remaining -= 1
                new_remaining -= 1
            elif line:
                raise CoverageModelError(f"unexpected diff line {line_number}: {line[:40]!r}")
            if old_remaining == 0 and new_remaining == 0:
                old_line = None
                new_line = None
            continue
        if line.startswith("--- "):
            old_path = _diff_path(line, "--- ")
            continue
        if line.startswith("+++ "):
            new_path = _diff_path(line, "+++ ")
            continue
    if old_remaining != 0 or new_remaining != 0:
        raise CoverageModelError("diff ended before the current hunk was complete")
    return changes


def _region_verdict(
    path: str,
    region: RegionKey,
    unit: CoverageIndex,
    build: CoverageIndex,
    itest_runs: list[CoverageIndex],
) -> tuple[str, dict[str, Any]]:
    unit_count = unit.count(path, region)
    build_count = build.count(path, region)
    itest_counts = [coverage.count(path, region) for coverage in itest_runs]
    metadata = {
        "unit_runtime": unit_count,
        "test_build": build_count,
        "itest_runtime": itest_counts,
    }
    if unit_count > 0 or build_count > 0 or all(count > 0 for count in itest_counts):
        return "covered", metadata
    if any(count > 0 for count in itest_counts):
        return "unstable", metadata
    return "uncovered", metadata


def classify_diff(
    changes: Iterable[DiffChange],
    sources: dict[str, SourceEntry],
    merged: CoverageIndex,
    unit: CoverageIndex,
    build: CoverageIndex,
    itest_runs: list[CoverageIndex],
) -> list[dict[str, Any]]:
    if not itest_runs:
        raise CoverageModelError("diff classification needs at least one itest process")
    records: list[dict[str, Any]] = []
    for change in changes:
        metadata: dict[str, Any] = {}
        if change.kind == "deleted":
            state = "deleted"
            region_count = 0
        else:
            source = sources.get(change.path)
            if source is None or source.classification == "excluded":
                state = "out-of-scope"
                region_count = 0
            elif change.path not in merged.files:
                state = "not-mapped"
                region_count = 0
            else:
                assert change.new_line is not None
                regions = sorted(
                    region
                    for region in merged.files[change.path].regions
                    if region.intersects_line(change.new_line)
                )
                region_count = len(regions)
                if not regions:
                    state = "no-region"
                else:
                    verdicts: list[str] = []
                    evidence: list[dict[str, Any]] = []
                    for region in regions:
                        verdict, counts = _region_verdict(
                            change.path, region, unit, build, itest_runs
                        )
                        verdicts.append(verdict)
                        evidence.append(
                            {
                                "start": [region.start_line, region.start_column],
                                "end": [region.end_line, region.end_column],
                                "kind": region.kind,
                                "verdict": verdict,
                                **counts,
                            }
                        )
                    metadata["regions"] = evidence
                    if "unstable" in verdicts:
                        state = "unstable"
                    elif all(verdict == "covered" for verdict in verdicts):
                        state = "covered"
                    elif all(verdict == "uncovered" for verdict in verdicts):
                        state = "uncovered"
                    else:
                        state = "partial"
        records.append(
            {
                "path": change.path,
                "old_line": change.old_line,
                "new_line": change.new_line,
                "state": state,
                "region_count": region_count,
                "metadata": metadata,
            }
        )
    return records


def diff_exit(records: Iterable[dict[str, Any]]) -> tuple[int, str]:
    states = [record["state"] for record in records]
    if "not-mapped" in states:
        return 2, "ERROR coverage-diff: incomplete evidence"
    if any(state in {"partial", "uncovered", "unstable"} for state in states):
        return 1, "FAIL coverage-diff: uncovered, partial, or unstable changed regions"
    if "covered" in states:
        return 0, "PASS coverage-diff: changed executable regions covered"
    return 0, "SKIP coverage-diff: no in-scope executable regions changed"


def state_counts(records: Iterable[dict[str, Any]]) -> dict[str, int]:
    counts = {state: 0 for state in DIFF_STATES}
    for record in records:
        state = record.get("state")
        if state not in counts:
            raise CoverageModelError(f"unknown diff state: {state!r}")
        counts[state] += 1
    return counts


def parse_flush_sentinel(path: Path, expected_pid: int) -> dict[str, Any]:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CoverageModelError(f"invalid coverage flush sentinel {path}: {error}") from error
    if not isinstance(document, dict) or set(document) != {
        "schema_version",
        "pid",
        "stage",
        "status",
    }:
        raise CoverageModelError("coverage flush sentinel has invalid fields")
    if document != {
        "schema_version": 1,
        "pid": expected_pid,
        "stage": "scene",
        "status": 0,
    }:
        raise CoverageModelError(f"coverage flush sentinel is invalid: {document!r}")
    return document


def parse_itest_report(path: Path, schema_path: Path) -> tuple[dict[str, Any], bool]:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CoverageModelError(f"invalid Tier-1 report {path}: {error}") from error
    required = {
        "$schema",
        "schema_version",
        "run_id",
        "runner_version",
        "complete",
        "outcome",
        "environment",
        "selection",
        "repeat",
        "timeout_frames",
        "summary",
        "tests",
        "errors",
        "artifacts",
        "metadata",
    }
    if (
        not isinstance(document, dict)
        or set(document) != required
        or document.get("$schema") != "itest-report-v1.schema.json"
        or document.get("schema_version") != 1
    ):
        raise CoverageModelError("Tier-1 report has an invalid schema version")
    validate_json_document(document, schema_path, "Tier-1 report")
    selection = _json_object(document.get("selection"), "Tier-1 selection")
    summary = _json_object(document.get("summary"), "Tier-1 summary")
    registered = _nonnegative_integer(selection.get("registered"), "registered tests")
    selected = _nonnegative_integer(selection.get("selected"), "selected tests")
    skipped = _nonnegative_integer(summary.get("skipped"), "skipped tests")
    total = _nonnegative_integer(summary.get("total"), "total tests")
    passed = _nonnegative_integer(summary.get("passed"), "passed tests")
    failed = _nonnegative_integer(summary.get("failed"), "failed tests")
    flaky = _nonnegative_integer(summary.get("flaky"), "flaky tests")
    repeat = _positive_integer(document.get("repeat"), "Tier-1 repeat")
    tests = _json_array(document.get("tests"), "Tier-1 tests")
    errors = _json_array(document.get("errors"), "Tier-1 errors")
    environment = _json_object(document.get("environment"), "Tier-1 environment")
    timeout_frames = _positive_integer(
        document.get("timeout_frames"), "Tier-1 timeout frames"
    )
    names = [test.get("name") for test in tests if isinstance(test, dict)]
    ids = [test.get("id") for test in tests if isinstance(test, dict)]
    outcome_counts = {
        outcome: sum(
            isinstance(test, dict) and test.get("outcome") == outcome
            for test in tests
        )
        for outcome in ("pass", "fail", "flaky", "skip")
    }
    attempts_passed = 0
    attempts_failed = 0
    attempts_valid = True
    for test in tests:
        if not isinstance(test, dict):
            attempts_valid = False
            continue
        attempts = test.get("attempts")
        if not isinstance(attempts, list) or len(attempts) != 1:
            attempts_valid = False
            continue
        attempt = attempts[0]
        if not isinstance(attempt, dict) or attempt.get("index") != 1:
            attempts_valid = False
            continue
        attempt_outcome = attempt.get("outcome")
        failures = attempt.get("failures")
        if not isinstance(failures, list):
            attempts_valid = False
            continue
        attempts_passed += attempt_outcome == "pass"
        attempts_failed += attempt_outcome == "fail"
        if (
            test.get("outcome") != attempt_outcome
            or (attempt_outcome == "pass" and failures)
            or (attempt_outcome == "fail" and not failures)
            or any(
                isinstance(failure, dict) and failure.get("kind") == "timeout"
                for failure in failures
            )
        ):
            attempts_valid = False
    complete_selection = (
        registered > 0
        and selected == registered
        and selection.get("focus_run") is False
        and selection.get("filter") is None
        and selection.get("patterns") == []
        and repeat == 1
        and skipped == 0
        and total == selected
        and len(tests) == total
        and passed + failed + flaky + skipped == total
        and len(names) == len(tests)
        and len(names) == len(set(names))
        and len(ids) == len(tests)
        and len(ids) == len(set(ids))
        and outcome_counts
        == {"pass": passed, "fail": failed, "flaky": flaky, "skip": skipped}
        and summary.get("attempts_passed") == attempts_passed
        and summary.get("attempts_failed") == attempts_failed
        and attempts_valid
        and environment.get("build_profile") == "debug"
        and environment.get("debug_assertions") is True
        and timeout_frames == 600
        and not errors
    )
    if not complete_selection:
        raise CoverageModelError(
            "Tier-1 coverage workload selection or result semantics are invalid"
        )
    complete = document.get("complete") is True
    outcome = document.get("outcome")
    if not complete or outcome not in {"pass", "fail"}:
        raise CoverageModelError("Tier-1 report is incomplete or erroneous")
    if (outcome == "pass") != (failed == 0 and flaky == 0 and passed == total):
        raise CoverageModelError("Tier-1 outcome conflicts with summary counts")
    normalized = {
        "complete": complete,
        "outcome": outcome,
        "registered": registered,
        "selected": selected,
        "focus": False,
        "filter": None,
        "repeat": 1,
        "skipped": skipped,
    }
    return normalized, outcome == "pass"


def witness_line(repository: Path, witness: WitnessConfig) -> int:
    lines = (repository / witness.source).read_text(encoding="utf-8").splitlines()
    matches = [index for index, line in enumerate(lines, 1) if witness.line_contains in line]
    if len(matches) != 1:
        raise CoverageModelError(f"witness marker drifted: {witness.id}")
    return matches[0]


def evaluate_witness(
    repository: Path,
    witness: WitnessConfig,
    merged: CoverageIndex,
    unit: CoverageIndex,
    itest_runs: list[CoverageIndex],
) -> dict[str, Any]:
    line = witness_line(repository, witness)
    mapped = merged.files.get(witness.source)
    if mapped is None:
        raise CoverageModelError(f"witness source is not mapped: {witness.source}")
    regions = [region for region in mapped.regions if region.intersects_line(line)]
    if not regions:
        raise CoverageModelError(f"witness line has no executable region: {witness.id}")
    unit_count = max(unit.count(witness.source, region) for region in regions)
    itest_counts = [
        max(run.count(witness.source, region) for region in regions) for run in itest_runs
    ]
    unit_pass = (unit_count == 0) if witness.unit_runtime == "zero" else (unit_count > 0)
    itest_pass = (
        all(count == 0 for count in itest_counts)
        if witness.itest_runtime == "zero"
        else all(count > 0 for count in itest_counts)
    )
    return {
        "id": witness.id,
        "source": witness.source,
        "function": witness.function,
        "line": line,
        "test": witness.test,
        "unit_runtime": unit_count,
        "itest_runtime": itest_counts,
        "passed": unit_pass and itest_pass,
    }


SCHEMA_KEYWORDS = {
    "$defs",
    "$id",
    "$ref",
    "$schema",
    "additionalProperties",
    "const",
    "enum",
    "items",
    "maxItems",
    "minItems",
    "minLength",
    "minimum",
    "pattern",
    "properties",
    "required",
    "title",
    "type",
    "uniqueItems",
}


def _schema_type(value: Any, expected: str) -> bool:
    return {
        "null": value is None,
        "boolean": isinstance(value, bool),
        "integer": isinstance(value, int) and not isinstance(value, bool),
        "number": (
            isinstance(value, (int, float))
            and not isinstance(value, bool)
            and math.isfinite(value)
        ),
        "string": isinstance(value, str),
        "array": isinstance(value, list),
        "object": isinstance(value, dict),
    }.get(expected, False)


def _resolve_schema(root: dict[str, Any], reference: str) -> dict[str, Any]:
    if not reference.startswith("#/"):
        raise CoverageModelError(f"unsupported schema reference {reference!r}")
    node: Any = root
    for part in reference[2:].split("/"):
        try:
            node = node[part.replace("~1", "/").replace("~0", "~")]
        except (KeyError, TypeError) as error:
            raise CoverageModelError(f"unresolved schema reference {reference!r}") from error
    if not isinstance(node, dict):
        raise CoverageModelError(f"schema reference is not an object: {reference!r}")
    return node


def _validate_schema(
    value: Any, rule: dict[str, Any], root: dict[str, Any], path: str
) -> list[str]:
    unsupported = set(rule) - SCHEMA_KEYWORDS
    if unsupported:
        raise CoverageModelError(f"{path}: unsupported schema keywords {sorted(unsupported)!r}")
    if "$ref" in rule:
        return _validate_schema(value, _resolve_schema(root, rule["$ref"]), root, path)
    errors: list[str] = []
    expected = rule.get("type")
    if expected is not None:
        types = [expected] if isinstance(expected, str) else expected
        if not isinstance(types, list) or not all(isinstance(item, str) for item in types):
            raise CoverageModelError(f"{path}: invalid schema type")
        if not any(_schema_type(value, item) for item in types):
            return [f"{path}: expected {' or '.join(types)}"]
    if "const" in rule and value != rule["const"]:
        errors.append(f"{path}: expected constant {rule['const']!r}")
    if "enum" in rule and value not in rule["enum"]:
        errors.append(f"{path}: expected one of {rule['enum']!r}")
    if isinstance(value, dict):
        for key in rule.get("required", []):
            if key not in value:
                errors.append(f"{path}: missing required property {key!r}")
        properties = rule.get("properties", {})
        for key, child in value.items():
            if key in properties:
                errors.extend(_validate_schema(child, properties[key], root, f"{path}.{key}"))
            elif rule.get("additionalProperties") is False:
                errors.append(f"{path}: unexpected property {key!r}")
            elif isinstance(rule.get("additionalProperties"), dict):
                errors.extend(
                    _validate_schema(
                        child, rule["additionalProperties"], root, f"{path}.{key}"
                    )
                )
    if isinstance(value, list):
        if len(value) < rule.get("minItems", 0):
            errors.append(f"{path}: expected at least {rule['minItems']} items")
        if "maxItems" in rule and len(value) > rule["maxItems"]:
            errors.append(f"{path}: expected at most {rule['maxItems']} items")
        if rule.get("uniqueItems"):
            encoded = [json.dumps(item, sort_keys=True, separators=(",", ":")) for item in value]
            if len(encoded) != len(set(encoded)):
                errors.append(f"{path}: expected unique items")
        if "items" in rule:
            for index, item in enumerate(value):
                errors.extend(_validate_schema(item, rule["items"], root, f"{path}[{index}]"))
    if isinstance(value, str):
        if len(value) < rule.get("minLength", 0):
            errors.append(f"{path}: string is too short")
        if "pattern" in rule and re.search(rule["pattern"], value) is None:
            errors.append(f"{path}: does not match {rule['pattern']!r}")
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in rule and value < rule["minimum"]:
            errors.append(f"{path}: expected value >= {rule['minimum']}")
    return errors


def validate_json_document(document: Any, schema_path: Path, label: str) -> None:
    try:
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CoverageModelError(f"could not load {label} schema: {error}") from error
    schema = _json_object(schema, f"{label} schema")
    errors = _validate_schema(document, schema, schema, "$")
    if errors:
        raise CoverageModelError(f"{label} schema validation failed:\n" + "\n".join(errors))


def validate_coverage_document(document: dict[str, Any], schema_path: Path) -> None:
    try:
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CoverageModelError(f"could not load coverage schema: {error}") from error
    schema = _json_object(schema, "coverage schema")
    errors = _validate_schema(document, schema, schema, "$")
    complete = document.get("complete")
    outcome = document.get("outcome")
    if complete is True and outcome in {"incomplete", "error"}:
        errors.append("$: complete evidence cannot be incomplete or erroneous")
    if complete is False and outcome in {"pass", "fail", "skip"}:
        errors.append("$: incomplete evidence cannot pass, fail, or skip")
    if complete is True and document.get("errors"):
        errors.append("$: complete evidence cannot contain errors")
    mode = document.get("mode")
    if mode == "full" and document.get("diff") is not None:
        errors.append("$: full evidence cannot contain a diff")
    if mode == "diff" and document.get("diff") is None:
        errors.append("$: diff evidence needs a diff record")
    if document.get("sources") != list(COVERAGE_SOURCES):
        errors.append("$.sources: coverage source order is invalid")
    if document.get("rate_gates") != []:
        errors.append("$.rate_gates: rate gates are forbidden")
    scope = document.get("scope", {})
    scope_files = scope.get("files", []) if isinstance(scope, dict) else []
    if not isinstance(scope_files, list):
        scope_files = []
    included: list[dict[str, Any]] = []
    paths = [record.get("path") for record in scope_files if isinstance(record, dict)]
    if len(paths) != len(set(paths)):
        errors.append("$.scope.files: duplicate source path")
    if isinstance(scope, dict):
        summary = scope.get("summary", {})
        included = [
            record
            for record in scope_files
            if isinstance(record, dict) and record.get("classification") == "included"
        ]
        excluded = [
            record
            for record in scope_files
            if isinstance(record, dict) and record.get("classification") == "excluded"
        ]
        mapped = [record for record in included if record.get("mapping") == "mapped"]
        unmapped = [record for record in included if record.get("mapping") == "unmapped"]
        expected_summary = {
            "all_rust_files": len(scope_files),
            "included": len(included),
            "excluded": len(excluded),
            "mapped": len(mapped),
            "unmapped": len(unmapped),
            "source_lines": sum(
                record.get("source_lines", 0)
                for record in scope_files
                if isinstance(record, dict)
            ),
        }
        if summary != expected_summary:
            errors.append("$.scope.summary: counts do not match the source ledger")
        if any(record.get("mapping") is not None for record in excluded):
            errors.append("$.scope.files: excluded sources cannot have mappings")
        if complete is True and any(record.get("mapping") is None for record in included):
            errors.append("$.scope.files: complete included sources need mapping status")
    coverage_files = document.get("files", [])
    if not isinstance(coverage_files, list):
        coverage_files = []
    coverage_paths = [record.get("path") for record in coverage_files if isinstance(record, dict)]
    if len(coverage_paths) != len(set(coverage_paths)):
        errors.append("$.files: duplicate source path")
    if complete is True and coverage_paths != [record.get("path") for record in included]:
        errors.append("$.files: complete file ledger does not match included scope")
    objects = document.get("objects", {})
    if isinstance(objects, dict):
        object_records = objects.get("records", [])
        if isinstance(object_records, list) and objects.get("count") != len(object_records):
            errors.append("$.objects.count: count does not match records")
    phases = document.get("phases", [])
    if not isinstance(phases, list):
        phases = []
    phase_ids = [phase.get("id") for phase in phases if isinstance(phase, dict)]
    if len(phase_ids) != len(set(phase_ids)):
        errors.append("$.phases: duplicate phase")
    if complete is True and phase_ids != [
        "unit-build",
        "unit-runtime",
        "itest-build",
        "import",
        "itest-runtime",
    ]:
        errors.append("$.phases: complete evidence needs the exact phase sequence")
    reports = document.get("test_reports", [])
    if not isinstance(reports, list):
        reports = []
    if complete is True:
        expected_reports = 3 if mode == "diff" else 1
        if len(reports) != expected_reports:
            errors.append(f"$.test_reports: expected {expected_reports} complete reports")
        if any(report.get("complete") is not True for report in reports):
            errors.append("$.test_reports: incomplete Tier-1 reference")
    totals = document.get("totals", {})
    if isinstance(totals, dict):
        for source, counts in totals.items():
            if not isinstance(counts, dict):
                continue
            for metric, pair in counts.items():
                if isinstance(pair, dict) and pair.get("covered", 0) > pair.get("count", 0):
                    errors.append(f"$.totals.{source}.{metric}: covered exceeds count")
    artifacts = [
        record.get("kind") for record in document.get("artifacts", []) if isinstance(record, dict)
    ]
    if len(artifacts) != len(set(artifacts)):
        errors.append("$.artifacts: duplicate artifact kind")
    diff = document.get("diff")
    if isinstance(diff, dict):
        lines = diff.get("lines", [])
        actual = state_counts(lines)
        if diff.get("state_counts") != actual:
            errors.append("$.diff.state_counts: counts do not match lines")
        if actual["not-mapped"] and complete is True:
            errors.append("$.diff.lines: not-mapped evidence cannot be complete")
        verdict, terminal = diff_exit(lines)
        if verdict == 1 and outcome != "fail":
            errors.append("$.diff.lines: failing regions require a fail outcome")
        if verdict == 2 and outcome != "error":
            errors.append("$.diff.lines: incomplete regions require an error outcome")
        if outcome == "pass" and not terminal.startswith("PASS"):
            errors.append("$.diff.lines: pass outcome conflicts with diff verdict")
        if outcome == "skip" and not terminal.startswith("SKIP"):
            errors.append("$.diff.lines: skip outcome conflicts with diff verdict")
    if errors:
        raise CoverageModelError("coverage-v1 validation failed:\n" + "\n".join(errors))


def coverage_exit(document: dict[str, Any]) -> int:
    if document.get("complete") is not True:
        return 2
    outcome = document.get("outcome")
    if outcome in {"pass", "skip"}:
        return 0
    if outcome == "fail":
        return 1
    return 2
