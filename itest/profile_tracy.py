#!/usr/bin/env python3
from __future__ import annotations

import bisect
import csv
import math
import re
from collections import Counter, defaultdict, deque
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from profile_schema import DISCLOSURE

TSV_COLUMNS = [
    "name",
    "src_file",
    "src_line",
    "ns_since_start",
    "exec_time_ns",
    "thread",
    "value",
]
MARKER_PREFIX = "__gbprof::"
WARMUP_ITERATIONS = 5
SAMPLE_ITERATIONS = 21
P95_MIN_OCCURRENCES = 20
P99_MIN_OCCURRENCES = 100
HIGH_CARDINALITY_THRESHOLD = 100
MARKER_NAME = re.compile(r"^(__gbprof::[a-z_]+)(?:\{(.*)\})?$")


class AggregationError(ValueError):
    pass


@dataclass(frozen=True)
class ZoneIdentity:
    name: str
    source_file: str
    source_line: int
    start_ns: int
    thread: int
    value: str


@dataclass(frozen=True)
class Zone:
    identity: ZoneIdentity
    duration_ns: int

    @property
    def start_ns(self) -> int:
        return self.identity.start_ns

    @property
    def end_ns(self) -> int:
        return self.start_ns + self.duration_ns


@dataclass(frozen=True)
class Marker:
    kind: str
    fields: dict[str, str]
    zone: Zone


@dataclass(frozen=True)
class PairedZone:
    inclusive: Zone
    self_duration_ns: int


@dataclass(frozen=True)
class SampleWindow:
    benchmark: str
    iteration: int
    start_ns: int
    end_ns: int


def normalize_source_file(source_file: str, repository: Path | None = None) -> str:
    normalized = source_file.replace("\\", "/")
    if repository is not None:
        repository_text = str(repository.resolve()).replace("\\", "/").rstrip("/")
        if normalized.startswith(f"{repository_text}/"):
            return normalized[len(repository_text) + 1 :]
    normalized = re.sub(
        r"^.*/\.cargo/registry/src/[^/]+/([^/]+/.*)$", r"<cargo>/\1", normalized
    )
    normalized = re.sub(r"^/nix/store/[^/]+-source/(.*)$", r"<nix-source>/\1", normalized)
    return normalized


def parse_tsv(path: Path, repository: Path | None = None) -> list[Zone]:
    try:
        with path.open(newline="", encoding="utf-8") as handle:
            reader = csv.DictReader(handle, delimiter="\t")
            if reader.fieldnames != TSV_COLUMNS:
                raise AggregationError(
                    f"{path}: expected TSV columns {TSV_COLUMNS}, got {reader.fieldnames}"
                )
            zones = []
            for line_number, row in enumerate(reader, start=2):
                if None in row or any(row[column] is None for column in TSV_COLUMNS):
                    raise AggregationError(f"{path}:{line_number}: malformed TSV row")
                try:
                    source_line = int(row["src_line"])
                    start_ns = int(row["ns_since_start"])
                    duration_ns = int(row["exec_time_ns"])
                    thread = int(row["thread"])
                except ValueError as error:
                    raise AggregationError(
                        f"{path}:{line_number}: invalid integer field"
                    ) from error
                if min(source_line, start_ns, duration_ns, thread) < 0:
                    raise AggregationError(f"{path}:{line_number}: negative numeric field")
                if not row["name"] or not row["src_file"]:
                    raise AggregationError(f"{path}:{line_number}: empty name or source file")
                zones.append(
                    Zone(
                        ZoneIdentity(
                            name=row["name"],
                            source_file=normalize_source_file(row["src_file"], repository),
                            source_line=source_line,
                            start_ns=start_ns,
                            thread=thread,
                            value=row["value"],
                        ),
                        duration_ns,
                    )
                )
    except (OSError, UnicodeError, csv.Error) as error:
        raise AggregationError(f"failed to parse {path}: {error}") from error
    if not zones:
        raise AggregationError(f"{path}: export contains no zones")
    return zones


def pair_exports(inclusive: list[Zone], self_zones: list[Zone]) -> list[PairedZone]:
    inclusive_counts = Counter(zone.identity for zone in inclusive)
    self_counts = Counter(zone.identity for zone in self_zones)
    if inclusive_counts != self_counts:
        missing = list((inclusive_counts - self_counts).elements())[:3]
        extra = list((self_counts - inclusive_counts).elements())[:3]
        raise AggregationError(
            f"inclusive/self export identity mismatch: missing={missing} extra={extra}"
        )

    durations: dict[ZoneIdentity, deque[int]] = defaultdict(deque)
    for zone in self_zones:
        durations[zone.identity].append(zone.duration_ns)
    return [
        PairedZone(zone, durations[zone.identity].popleft()) for zone in inclusive
    ]


def parse_marker(zone: Zone) -> Marker | None:
    name = zone.identity.name
    if not name.startswith(MARKER_PREFIX):
        return None
    match = MARKER_NAME.fullmatch(name)
    if match is None:
        raise AggregationError(f"malformed profiling marker name {name!r}")
    fields: dict[str, str] = {}
    body = match.group(2)
    if body:
        for token in body.split():
            if "=" not in token:
                raise AggregationError(f"malformed profiling marker field {token!r}")
            key, value = token.split("=", 1)
            if not key or key in fields or not value:
                raise AggregationError(f"malformed profiling marker field {token!r}")
            fields[key] = value.strip('"')
    return Marker(match.group(1)[len(MARKER_PREFIX) :], fields, zone)


def _required_fields(marker: Marker, required: set[str]) -> None:
    if marker.fields.keys() != required:
        raise AggregationError(
            f"{MARKER_PREFIX}{marker.kind} fields must be {sorted(required)}, "
            f"got {sorted(marker.fields)}"
        )


def _single(markers: list[Marker], kind: str, **fields: str) -> Marker:
    matches = [
        marker
        for marker in markers
        if marker.kind == kind
        and all(marker.fields.get(name) == value for name, value in fields.items())
    ]
    if len(matches) != 1:
        raise AggregationError(
            f"expected one {MARKER_PREFIX}{kind} marker for {fields}, got {len(matches)}"
        )
    return matches[0]


def _contains(outer: Zone, inner: Zone) -> bool:
    return outer.start_ns <= inner.start_ns and inner.end_ns <= outer.end_ns


def _int_field(marker: Marker, field: str) -> int:
    try:
        value = int(marker.fields[field])
    except (KeyError, ValueError) as error:
        raise AggregationError(
            f"{MARKER_PREFIX}{marker.kind} has invalid {field} field"
        ) from error
    if value < 0:
        raise AggregationError(f"{MARKER_PREFIX}{marker.kind} has negative {field}")
    return value


def _validate_workload(
    workload: dict[str, Any], run_id: str
) -> tuple[dict[str, Any], dict[str, int]]:
    if workload.get("benchmark_compatible") is not False:
        raise AggregationError("profiled workload must set benchmark_compatible=false")
    if workload.get("disclosure") != DISCLOSURE:
        raise AggregationError("profiled workload disclosure is missing")
    if workload.get("profile_run_id") != run_id:
        raise AggregationError("profiled workload run ID does not match")

    selection = workload.get("selection")
    if not isinstance(selection, dict):
        raise AggregationError("profiled workload selection is missing")
    benchmarks = selection.get("benchmarks")
    if (
        selection.get("mode") not in {"exact", "filter"}
        or not isinstance(benchmarks, list)
        or not benchmarks
        or not all(isinstance(name, str) and name for name in benchmarks)
        or selection.get("selected") != len(benchmarks)
        or len(set(benchmarks)) != len(benchmarks)
    ):
        raise AggregationError("profiled workload selection is invalid")
    if selection["mode"] == "exact" and len(benchmarks) != 1:
        raise AggregationError("exact profile selection did not resolve to one benchmark")

    profiling = workload.get("profiling")
    if not isinstance(profiling, dict):
        raise AggregationError("profiled workload lifecycle metadata is missing")
    if (
        profiling.get("warmup_iterations") != WARMUP_ITERATIONS
        or profiling.get("sample_iterations") != SAMPLE_ITERATIONS
    ):
        raise AggregationError("profiled workload lifecycle counts do not match the contract")
    repetitions = profiling.get("inner_repetitions")
    if not isinstance(repetitions, dict) or repetitions.keys() != set(benchmarks):
        raise AggregationError("profiled workload repetition metadata is invalid")
    for name, value in repetitions.items():
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            raise AggregationError(f"invalid inner repetition count for {name}")
    return selection, repetitions


def _validate_markers(
    markers: list[Marker], run_id: str, benchmarks: list[str], repetitions: dict[str, int]
) -> tuple[list[SampleWindow], dict[str, bool]]:
    for marker in markers:
        required = {
            "run_begin": {"run_id"},
            "run_end": {"run_id"},
            "benchmark": {"benchmark", "inner_repetitions"},
            "phase": {"benchmark", "phase"},
            "iteration": {
                "benchmark",
                "phase",
                "iteration",
                "inner_repetitions",
            },
            "measured": {"benchmark", "phase", "iteration"},
        }.get(marker.kind)
        if required is None:
            raise AggregationError(f"unknown profiling marker {MARKER_PREFIX}{marker.kind}")
        _required_fields(marker, required)
        if marker.kind in {"phase", "iteration", "measured"}:
            phase = marker.fields["phase"]
            if phase not in {"warmup", "sample"}:
                raise AggregationError(f"invalid profiling phase {phase!r}")
        if marker.kind in {"iteration", "measured"}:
            iteration = _int_field(marker, "iteration")
            expected = (
                WARMUP_ITERATIONS
                if marker.fields["phase"] == "warmup"
                else SAMPLE_ITERATIONS
            )
            if iteration >= expected:
                raise AggregationError(
                    f"{MARKER_PREFIX}{marker.kind} iteration is outside its phase"
                )

    run_markers = [
        marker for marker in markers if marker.kind in {"run_begin", "run_end"}
    ]
    if (
        len(run_markers) != 2
        or Counter(marker.kind for marker in run_markers)
        != Counter({"run_begin": 1, "run_end": 1})
        or any(marker.fields["run_id"] != run_id for marker in run_markers)
    ):
        raise AggregationError("profile run boundary marker identity mismatch")

    run_begin = _single(markers, "run_begin", run_id=run_id)
    run_end = _single(markers, "run_end", run_id=run_id)
    if run_begin.zone.end_ns > run_end.zone.start_ns:
        raise AggregationError("profile run boundary markers are out of order")

    marker_benchmarks = {
        marker.fields["benchmark"] for marker in markers if "benchmark" in marker.fields
    }
    if marker_benchmarks != set(benchmarks):
        raise AggregationError(
            f"profile marker benchmark identity mismatch: {sorted(marker_benchmarks)}"
        )

    windows: list[SampleWindow] = []
    measured_by_benchmark: dict[str, bool] = {}
    benchmark_zones = []
    for benchmark in benchmarks:
        expected_repetitions = repetitions[benchmark]
        benchmark_marker = _single(markers, "benchmark", benchmark=benchmark)
        benchmark_zones.append(benchmark_marker.zone)
        if _int_field(benchmark_marker, "inner_repetitions") != expected_repetitions:
            raise AggregationError(f"inner repetition marker mismatch for {benchmark}")
        if not (
            run_begin.zone.end_ns <= benchmark_marker.zone.start_ns
            and benchmark_marker.zone.end_ns <= run_end.zone.start_ns
        ):
            raise AggregationError(f"benchmark marker is outside run boundaries for {benchmark}")

        phase_markers = {
            phase: _single(markers, "phase", benchmark=benchmark, phase=phase)
            for phase in ("warmup", "sample")
        }
        if phase_markers["warmup"].zone.end_ns > phase_markers["sample"].zone.start_ns:
            raise AggregationError(f"benchmark phases are out of order for {benchmark}")

        measured_counts = []
        for phase, expected_count in (
            ("warmup", WARMUP_ITERATIONS),
            ("sample", SAMPLE_ITERATIONS),
        ):
            phase_marker = phase_markers[phase]
            if not _contains(benchmark_marker.zone, phase_marker.zone):
                raise AggregationError(f"{phase} phase is outside benchmark marker for {benchmark}")
            iterations = [
                marker
                for marker in markers
                if marker.kind == "iteration"
                and marker.fields["benchmark"] == benchmark
                and marker.fields["phase"] == phase
            ]
            indices = sorted(_int_field(marker, "iteration") for marker in iterations)
            if indices != list(range(expected_count)):
                raise AggregationError(
                    f"{benchmark} {phase} iterations must be 0..{expected_count - 1}"
                )
            ordered_iterations = sorted(
                iterations, key=lambda marker: _int_field(marker, "iteration")
            )
            if any(
                current.zone.start_ns < previous.zone.end_ns
                for previous, current in zip(
                    ordered_iterations, ordered_iterations[1:]
                )
            ):
                raise AggregationError(
                    f"{benchmark} {phase} iteration markers overlap or are out of order"
                )
            for iteration_marker in iterations:
                iteration = _int_field(iteration_marker, "iteration")
                if _int_field(iteration_marker, "inner_repetitions") != expected_repetitions:
                    raise AggregationError(
                        f"inner repetition iteration marker mismatch for {benchmark}"
                    )
                if not _contains(phase_marker.zone, iteration_marker.zone):
                    raise AggregationError(
                        f"iteration marker is outside {phase} phase for {benchmark}"
                    )
                measured = [
                    marker
                    for marker in markers
                    if marker.kind == "measured"
                    and marker.fields["benchmark"] == benchmark
                    and marker.fields["phase"] == phase
                    and _int_field(marker, "iteration") == iteration
                ]
                if any(not _contains(iteration_marker.zone, marker.zone) for marker in measured):
                    raise AggregationError(
                        f"measured marker is outside iteration {iteration} for {benchmark}"
                    )
                if phase == "sample":
                    measured_counts.append(len(measured))
                    selected_windows = [marker.zone for marker in measured] or [
                        iteration_marker.zone
                    ]
                    windows.extend(
                        SampleWindow(
                            benchmark,
                            iteration,
                            zone.start_ns,
                            zone.end_ns,
                        )
                        for zone in selected_windows
                    )
        if not (
            all(count == 0 for count in measured_counts)
            or all(count > 0 for count in measured_counts)
        ):
            raise AggregationError(
                f"measured markers are inconsistent across samples for {benchmark}"
            )
        measured_by_benchmark[benchmark] = all(count > 0 for count in measured_counts)

    benchmark_zones.sort(key=lambda zone: zone.start_ns)
    if any(
        current.start_ns < previous.end_ns
        for previous, current in zip(benchmark_zones, benchmark_zones[1:])
    ):
        raise AggregationError("profile benchmark markers overlap")

    windows.sort(key=lambda window: (window.start_ns, window.end_ns))
    for previous, current in zip(windows, windows[1:]):
        if current.start_ns < previous.end_ns:
            raise AggregationError("profile sample windows overlap")
    return windows, measured_by_benchmark


def nearest_rank(values: list[int], percentile: float) -> int:
    if not values:
        raise AggregationError("cannot calculate a quantile of an empty sequence")
    ordered = sorted(values)
    return ordered[max(0, math.ceil(percentile * len(ordered)) - 1)]


def _timing(values_by_sample: list[list[int]], inner_repetitions: int) -> dict[str, Any]:
    values = [value for sample in values_by_sample for value in sample]
    if not values:
        raise AggregationError("cannot summarize an empty span")
    occurrence_count = len(values)

    def quantile(percentile: float, minimum: int) -> tuple[int | None, str | None]:
        if occurrence_count < minimum:
            return None, f"requires at least {minimum} occurrences"
        return nearest_rank(values, percentile), None

    p95_ns, p95_reason = quantile(0.95, P95_MIN_OCCURRENCES)
    p99_ns, p99_reason = quantile(0.99, P99_MIN_OCCURRENCES)
    return {
        "occurrence_count": occurrence_count,
        "total_ns": sum(values),
        "mean_ns": sum(values) / occurrence_count,
        "median_ns": nearest_rank(values, 0.5),
        "p95_ns": p95_ns,
        "p95_reason": p95_reason,
        "p99_ns": p99_ns,
        "p99_reason": p99_reason,
        "per_sample": [
            {
                "iteration": iteration,
                "normalized_total_ns": sum(sample) / inner_repetitions,
                "normalized_count": len(sample) / inner_repetitions,
            }
            for iteration, sample in enumerate(values_by_sample)
        ],
    }


def aggregate_exports(
    inclusive_path: Path,
    self_path: Path,
    workload: dict[str, Any],
    run_id: str,
    repository: Path | None = None,
) -> dict[str, Any]:
    selection, repetitions = _validate_workload(workload, run_id)
    benchmarks = selection["benchmarks"]
    paired = pair_exports(
        parse_tsv(inclusive_path, repository), parse_tsv(self_path, repository)
    )
    markers = [
        marker
        for pair in paired
        if (marker := parse_marker(pair.inclusive)) is not None
    ]
    windows, measured_by_benchmark = _validate_markers(
        markers, run_id, benchmarks, repetitions
    )

    window_starts = [window.start_ns for window in windows]
    grouped: dict[
        tuple[str, str, str, int], tuple[list[list[int]], list[list[int]]]
    ] = {}
    cardinality: dict[tuple[str, str, str, int], set[str]] = defaultdict(set)
    non_marker_counts = Counter()

    for pair in paired:
        zone = pair.inclusive
        if zone.identity.name.startswith(MARKER_PREFIX):
            continue
        index = bisect.bisect_right(window_starts, zone.start_ns) - 1
        if index < 0:
            continue
        window = windows[index]
        if zone.end_ns > window.end_ns:
            continue
        key = (
            window.benchmark,
            zone.identity.name,
            zone.identity.source_file,
            zone.identity.source_line,
        )
        if key not in grouped:
            grouped[key] = (
                [[] for _ in range(SAMPLE_ITERATIONS)],
                [[] for _ in range(SAMPLE_ITERATIONS)],
            )
        grouped[key][0][window.iteration].append(zone.duration_ns)
        grouped[key][1][window.iteration].append(pair.self_duration_ns)
        non_marker_counts[window.benchmark] += 1
        base_name = zone.identity.name.split("{", 1)[0]
        cardinality[
            (window.benchmark, base_name, zone.identity.source_file, zone.identity.source_line)
        ].add(zone.identity.name)

    missing = [name for name in benchmarks if non_marker_counts[name] == 0]
    if missing:
        raise AggregationError(f"no non-marker zones in measured samples for {missing}")

    spans = []
    for (benchmark, name, source_file, source_line), (inclusive, self_values) in sorted(
        grouped.items()
    ):
        spans.append(
            {
                "benchmark": benchmark,
                "name": name,
                "source_file": source_file,
                "source_line": source_line,
                "inclusive": _timing(inclusive, repetitions[benchmark]),
                "self": _timing(self_values, repetitions[benchmark]),
            }
        )

    warnings = []
    for (benchmark, base_name, source_file, source_line), emitted_names in sorted(
        cardinality.items()
    ):
        if len(emitted_names) > HIGH_CARDINALITY_THRESHOLD:
            warnings.append(
                {
                    "kind": "high-name-cardinality",
                    "message": f"{base_name} emitted {len(emitted_names)} distinct zone names",
                    "metadata": {
                        "benchmark": benchmark,
                        "source_file": source_file,
                        "source_line": source_line,
                    },
                }
            )

    workload_benchmarks = [
        {
            "name": benchmark,
            "inner_repetitions": repetitions[benchmark],
            "measured": measured_by_benchmark[benchmark],
        }
        for benchmark in benchmarks
    ]
    return {
        "selection": selection,
        "workload": {
            "kind": "itest-benchmark",
            "warmup_iterations": WARMUP_ITERATIONS,
            "sample_iterations": SAMPLE_ITERATIONS,
            "connection_gate": {
                "mechanism": "ondemand+Client::is_connected",
                "timeout_seconds": 10,
            },
            "benchmarks": workload_benchmarks,
        },
        "spans": spans,
        "warnings": warnings,
    }
