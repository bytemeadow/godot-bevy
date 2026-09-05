#!/usr/bin/env python3
from __future__ import annotations

import argparse
import gzip
import json
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterator


class GeckoProfileError(ValueError):
    pass


def load_json(path: Path) -> Any:
    try:
        with path.open("rb") as raw:
            compressed = raw.read(2) == b"\x1f\x8b"
        opener = gzip.open if compressed else open
        with opener(path, "rt", encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise GeckoProfileError(f"failed to read {path}: {error}") from error


def _column(table: Any, name: str, *, required: bool = True) -> list[Any]:
    if not isinstance(table, dict):
        raise GeckoProfileError(f"table containing {name!r} is not an object")
    value = table.get(name)
    if isinstance(value, list):
        return value
    schema = table.get("schema")
    data = table.get("data")
    if isinstance(schema, dict) and isinstance(schema.get(name), int) and isinstance(data, list):
        index = schema[name]
        try:
            return [row[index] for row in data]
        except (IndexError, TypeError) as error:
            raise GeckoProfileError(f"malformed row-oriented {name!r} column") from error
    if required:
        raise GeckoProfileError(f"missing required table column {name!r}")
    return []


def _checked_length(table: dict[str, Any], columns: list[list[Any]], label: str) -> int:
    lengths = {len(column) for column in columns}
    declared = table.get("length")
    if isinstance(declared, int) and not isinstance(declared, bool):
        lengths.add(declared)
    if len(lengths) != 1:
        raise GeckoProfileError(f"{label} columns have inconsistent lengths")
    return lengths.pop()


def _strings(thread: dict[str, Any]) -> list[str]:
    values = thread.get("stringArray", thread.get("stringTable"))
    if isinstance(values, dict):
        values = values.get("data")
    if not isinstance(values, list) or not all(isinstance(value, str) for value in values):
        raise GeckoProfileError("thread string table is missing or malformed")
    return values


def _text(value: Any, strings: list[str] | None) -> str | None:
    if isinstance(value, str) and value:
        return value
    if isinstance(value, int) and not isinstance(value, bool) and strings is not None:
        if 0 <= value < len(strings):
            return strings[value]
    return None


def _library_keys(node: dict[str, Any]) -> set[str]:
    fields = (
        "name",
        "path",
        "debugName",
        "debug_name",
        "breakpadId",
        "breakpad_id",
        "debugId",
        "debug_id",
        "codeId",
        "code_id",
    )
    return {
        value.strip().lower()
        for field in fields
        if isinstance((value := node.get(field)), str) and value.strip()
    }


def _local_strings(node: dict[str, Any], inherited: list[str] | None) -> list[str] | None:
    for field in ("string_table", "stringTable", "strings", "stringArray"):
        values = node.get(field)
        if isinstance(values, list) and all(isinstance(value, str) for value in values):
            return values
    return inherited


def _frame_names(value: Any, strings: list[str] | None) -> list[str]:
    if isinstance(value, list):
        names = []
        for item in value:
            names.extend(_frame_names(item, strings))
        return names
    if not isinstance(value, dict):
        return []
    names = []
    for field in ("function", "name"):
        if (name := _text(value.get(field), strings)) is not None:
            names.append(name)
    if names:
        return names
    nested = value.get("frames", value.get("inline_frames", value.get("inlines")))
    return _frame_names(nested, strings)


class SymbolResolver:
    def __init__(self, sidecar: Any):
        self._by_library: dict[
            str, list[tuple[int, int | None, tuple[str, ...]]]
        ] = defaultdict(list)
        self._global: dict[int, set[tuple[str, ...]]] = defaultdict(set)
        self._walk(sidecar, set(), None)
        for records in self._by_library.values():
            records.sort(key=lambda record: record[0])

    def _add(
        self,
        keys: set[str],
        address: Any,
        size: Any,
        names: list[str],
    ) -> None:
        if not isinstance(address, int) or isinstance(address, bool) or address < 0:
            return
        if not isinstance(size, int) or isinstance(size, bool) or size <= 0:
            size = None
        cleaned = [name for name in names if name]
        if not cleaned:
            return
        record = (address, size, tuple(cleaned))
        for key in keys:
            self._by_library[key].append(record)
        self._global[address].add(record[2])

    def _walk(
        self,
        node: Any,
        inherited_keys: set[str],
        inherited_strings: list[str] | None,
    ) -> None:
        if isinstance(node, list):
            for item in node:
                self._walk(item, inherited_keys, inherited_strings)
            return
        if not isinstance(node, dict):
            return

        strings = _local_strings(node, inherited_strings)
        keys = inherited_keys | _library_keys(node)
        rvas = node.get("rvas")
        if isinstance(rvas, list):
            frames = node.get("frames", [])
            symbols = node.get("symbols", node.get("symbol"))
            parallel_frames = (
                isinstance(frames, list)
                and len(frames) == len(rvas)
                and any(isinstance(item, (list, dict)) for item in frames)
            )
            for index, address in enumerate(rvas):
                frame_value = frames[index] if parallel_frames else frames
                names = _frame_names(frame_value, strings)
                symbol_value = (
                    symbols[index]
                    if isinstance(symbols, list) and len(symbols) == len(rvas)
                    else symbols
                )
                if not names and (name := _text(symbol_value, strings)) is not None:
                    names = [name]
                sizes = node.get("sizes", node.get("symbol_sizes", []))
                size = (
                    sizes[index]
                    if isinstance(sizes, list) and len(sizes) == len(rvas)
                    else None
                )
                self._add(keys, address, size, names)

        address = node.get("rva", node.get("address", node.get("moduleOffset")))
        if address is not None:
            names = _frame_names(node, strings)
            if not names and (name := _text(node.get("symbol"), strings)) is not None:
                names = [name]
            self._add(keys, address, node.get("size", node.get("functionSize")), names)

        for field, child in node.items():
            if field not in {"string_table", "stringTable", "strings", "stringArray"}:
                self._walk(child, keys, strings)

    @staticmethod
    def _resolve_records(
        records: list[tuple[int, int | None, tuple[str, ...]]], address: int
    ) -> set[tuple[str, ...]]:
        low = 0
        high = len(records)
        while low < high:
            midpoint = (low + high) // 2
            if address < records[midpoint][0]:
                high = midpoint
            else:
                low = midpoint + 1
        index = low - 1
        if index < 0:
            return set()
        start, size, names = records[index]
        next_start = records[index + 1][0] if index + 1 < len(records) else None
        if size is not None:
            return {names} if address < start + size else set()
        if address == start or next_start is not None and address < next_start:
            return {names}
        return set()

    def resolve(self, library: dict[str, Any] | None, address: int) -> list[str] | None:
        if library is not None:
            candidates: set[tuple[str, ...]] = set()
            for key in _library_keys(library):
                candidates.update(
                    self._resolve_records(self._by_library.get(key, []), address)
                )
            if len(candidates) == 1:
                return list(next(iter(candidates)))
            return None
        candidates = self._global.get(address, set())
        if len(candidates) == 1:
            return list(next(iter(candidates)))
        return None


def _threads(profile: dict[str, Any]) -> Iterator[tuple[str, dict[str, Any]]]:
    def walk(node: Any, process_name: str) -> Iterator[tuple[str, dict[str, Any]]]:
        if not isinstance(node, dict):
            return
        current_name = str(
            node.get("processName", node.get("name", process_name)) or process_name
        )
        threads = node.get("threads")
        if isinstance(threads, list):
            for thread in threads:
                if isinstance(thread, dict):
                    yield current_name, thread
                else:
                    raise GeckoProfileError("profile thread entry is not an object")
        processes = node.get("processes")
        if isinstance(processes, list):
            for process in processes:
                yield from walk(process, current_name)

    meta = profile.get("meta", {})
    fallback = meta.get("product", "process") if isinstance(meta, dict) else "process"
    yield from walk(profile, str(fallback))


def _sanitize_frame(value: str) -> str:
    return " ".join(value.replace(";", ":").split()) or "[unknown]"


UNKNOWN_FRAME = re.compile(
    r"^(?:\[?<unknown>\]?|\[?unknown\]?|<unknown(?: in .*)?>|0x[0-9a-f]+|"
    r"[0-9a-f]{6,}|.*\+\s*0x[0-9a-f]+)$",
    re.IGNORECASE,
)


def _is_unknown(name: str) -> bool:
    return bool(UNKNOWN_FRAME.fullmatch(name.strip()))


def _symbol_presence(names: set[str]) -> tuple[bool, bool]:
    rust_patterns = ("godot_bevy", "godot-bevy", "bevy_", "bevy::", "core::", "std::", "alloc::")
    godot_patterns = (
        "Godot",
        "SceneTree::",
        "Main::",
        "Node::",
        "Object::",
        "RenderingServer::",
        "PhysicsServer",
    )
    return (
        any(any(pattern in name for pattern in rust_patterns) for name in names),
        any(any(pattern in name for pattern in godot_patterns) for name in names),
    )


@dataclass(frozen=True)
class ConversionResult:
    folded: Counter[tuple[str, ...]]
    sample_count: int
    unknown_leaf_count: int
    rust_symbols: bool
    godot_symbols: bool
    leaf_counts: Counter[str]

    @property
    def unknown_leaf_ratio(self) -> float:
        return self.unknown_leaf_count / self.sample_count


def convert_profile(profile: Any, sidecar: Any | None = None) -> ConversionResult:
    if not isinstance(profile, dict):
        raise GeckoProfileError("Gecko profile root must be an object")
    libraries = profile.get("libs", [])
    if not isinstance(libraries, list) or not all(isinstance(lib, dict) for lib in libraries):
        raise GeckoProfileError("profile library table is malformed")
    resolver = SymbolResolver(sidecar) if sidecar is not None else None
    folded: Counter[tuple[str, ...]] = Counter()
    leaves: Counter[str] = Counter()
    symbols: set[str] = set()
    unknown = 0
    total = 0
    thread_count = 0

    for process_name, thread in _threads(profile):
        thread_count += 1
        strings = _strings(thread)
        sample_table = thread.get("samples")
        stack_indices = _column(sample_table, "stack")
        weights = _column(sample_table, "weight", required=False)
        if not weights:
            weights = [1] * len(stack_indices)
        _checked_length(sample_table, [stack_indices, weights], "sample table")

        stack_table = thread.get("stackTable")
        prefixes = _column(stack_table, "prefix")
        frame_indices = _column(stack_table, "frame")
        stack_length = _checked_length(
            stack_table, [prefixes, frame_indices], "stack table"
        )
        frame_table = thread.get("frameTable")
        functions = _column(frame_table, "func")
        addresses = _column(frame_table, "address", required=False)
        frame_lib_indices = _column(frame_table, "libIndex", required=False)
        native_symbol_indices = _column(
            frame_table, "nativeSymbol", required=False
        )
        if not addresses:
            addresses = [None] * len(functions)
        if not frame_lib_indices:
            frame_lib_indices = [None] * len(functions)
        if not native_symbol_indices:
            native_symbol_indices = [None] * len(functions)
        frame_length = _checked_length(
            frame_table,
            [functions, addresses, frame_lib_indices, native_symbol_indices],
            "frame table",
        )
        func_table = thread.get("funcTable")
        function_names = _column(func_table, "name")
        function_resources = _column(func_table, "resource", required=False)
        if not function_resources:
            function_resources = [None] * len(function_names)
        _checked_length(
            func_table, [function_names, function_resources], "function table"
        )

        resource_table = thread.get("resourceTable")
        resource_libraries = (
            _column(resource_table, "lib", required=False)
            if resource_table is not None
            else []
        )
        if resource_table is not None:
            _checked_length(resource_table, [resource_libraries], "resource table")

        native_symbols = thread.get("nativeSymbols")
        native_symbol_libraries = (
            _column(native_symbols, "libIndex", required=False)
            if native_symbols is not None
            else []
        )
        if native_symbols is not None:
            _checked_length(
                native_symbols, [native_symbol_libraries], "native symbol table"
            )

        def library_for_frame(frame_index: int, function_index: int) -> dict[str, Any] | None:
            lib_index = frame_lib_indices[frame_index]
            native_symbol_index = native_symbol_indices[frame_index]
            if isinstance(native_symbol_index, int) and not isinstance(
                native_symbol_index, bool
            ):
                if not 0 <= native_symbol_index < len(native_symbol_libraries):
                    raise GeckoProfileError(
                        "frame references a symbol outside nativeSymbols"
                    )
                lib_index = native_symbol_libraries[native_symbol_index]
            if lib_index is None or lib_index == -1:
                resource_index = function_resources[function_index]
                if isinstance(resource_index, int) and not isinstance(
                    resource_index, bool
                ):
                    if resource_index == -1:
                        return None
                    if not 0 <= resource_index < len(resource_libraries):
                        raise GeckoProfileError(
                            "function references a resource outside resourceTable"
                        )
                    lib_index = resource_libraries[resource_index]
            if lib_index is None or lib_index == -1:
                return None
            if not isinstance(lib_index, int) or isinstance(lib_index, bool):
                raise GeckoProfileError("library index is not an integer")
            if not 0 <= lib_index < len(libraries):
                raise GeckoProfileError("frame references a library outside libs")
            return libraries[lib_index]

        def frame_names(frame_index: int) -> list[str]:
            if not 0 <= frame_index < frame_length:
                raise GeckoProfileError("stack references a frame outside frameTable")
            function_index = functions[frame_index]
            if not isinstance(function_index, int) or isinstance(function_index, bool):
                raise GeckoProfileError("frame func index is not an integer")
            if not 0 <= function_index < len(function_names):
                raise GeckoProfileError("frame references a function outside funcTable")
            direct = _text(function_names[function_index], strings) or "[unknown]"
            address = addresses[frame_index]
            library = library_for_frame(frame_index, function_index)
            if (
                resolver is not None
                and isinstance(address, int)
                and not isinstance(address, bool)
                and address >= 0
            ):
                if resolved := resolver.resolve(library, address):
                    return [_sanitize_frame(name) for name in resolved]
            return [_sanitize_frame(direct)]

        process_label = _sanitize_frame(
            str(thread.get("processName", process_name) or process_name)
        )
        thread_label = _sanitize_frame(
            str(thread.get("name", thread.get("tid", "thread")) or "thread")
        )
        for stack_index, weight_value in zip(stack_indices, weights):
            if (
                not isinstance(weight_value, int)
                or isinstance(weight_value, bool)
                or weight_value <= 0
            ):
                raise GeckoProfileError("sample weight must be a positive integer")
            if stack_index is None:
                names = ["[unknown]"]
            else:
                if not isinstance(stack_index, int) or isinstance(stack_index, bool):
                    raise GeckoProfileError("sample stack index is not an integer")
                chain = []
                visited = set()
                current: int | None = stack_index
                while current is not None:
                    if not isinstance(current, int) or not 0 <= current < stack_length:
                        raise GeckoProfileError("sample references a stack outside stackTable")
                    if current in visited:
                        raise GeckoProfileError("stackTable prefix cycle detected")
                    visited.add(current)
                    frame_index = frame_indices[current]
                    if not isinstance(frame_index, int) or isinstance(frame_index, bool):
                        raise GeckoProfileError("stack frame index is not an integer")
                    chain.append(frame_index)
                    current = prefixes[current]
                names = []
                for frame_index in reversed(chain):
                    names.extend(frame_names(frame_index))
                if not names:
                    names = ["[unknown]"]
            leaf = names[-1]
            stack = (f"process:{process_label}", f"thread:{thread_label}", *names)
            folded[stack] += weight_value
            leaves[leaf] += weight_value
            symbols.update(names)
            total += weight_value
            if _is_unknown(leaf):
                unknown += weight_value

    if thread_count == 0:
        raise GeckoProfileError("profile contains no threads")
    if total == 0 or not folded:
        raise GeckoProfileError("profile contains no stack samples")
    rust_symbols, godot_symbols = _symbol_presence(symbols)
    return ConversionResult(
        folded,
        total,
        unknown,
        rust_symbols,
        godot_symbols,
        leaves,
    )


def write_folded(result: ConversionResult, output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as handle:
        for stack, count in sorted(result.folded.items()):
            handle.write(f"{';'.join(stack)} {count}\n")


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Convert a Samply processed Gecko profile to folded stacks."
    )
    parser.add_argument("profile", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--symbols", type=Path, help="Samply .syms.json sidecar")
    return parser


def main() -> int:
    args = create_parser().parse_args()
    try:
        profile = load_json(args.profile)
        sidecar = load_json(args.symbols) if args.symbols is not None else None
        result = convert_profile(profile, sidecar)
        write_folded(result, args.output)
    except (GeckoProfileError, OSError) as error:
        print(f"Conversion failed: {error}", file=sys.stderr)
        return 2
    print(
        f"Converted {result.sample_count} samples; "
        f"unknown leaves {result.unknown_leaf_ratio:.1%}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
