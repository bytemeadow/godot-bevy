#!/usr/bin/env python3
from __future__ import annotations

import ast
import re
from pathlib import Path
from typing import Any

try:
    import tomllib
except ImportError:
    tomllib = None


class TomlError(ValueError):
    pass


def _strip_comment(line: str) -> str:
    quoted = False
    escaped = False
    for index, character in enumerate(line):
        if escaped:
            escaped = False
        elif character == "\\" and quoted:
            escaped = True
        elif character == '"':
            quoted = not quoted
        elif character == "#" and not quoted:
            return line[:index]
    return line


def _parse_value(value: str) -> Any:
    value = value.strip()
    if value in {"true", "false"}:
        return value == "true"
    if value.startswith('"'):
        try:
            parsed = ast.literal_eval(value)
        except (SyntaxError, ValueError) as error:
            raise TomlError(f"invalid TOML string {value!r}") from error
        if not isinstance(parsed, str):
            raise TomlError(f"expected TOML string, got {value!r}")
        return parsed
    if value.startswith("["):
        try:
            parsed = ast.literal_eval(value)
        except (SyntaxError, ValueError) as error:
            raise TomlError(f"invalid TOML array {value!r}") from error
        if not isinstance(parsed, list):
            raise TomlError(f"expected TOML array, got {value!r}")
        return parsed
    if re.fullmatch(r"[+-]?\d+", value):
        return int(value)
    if re.fullmatch(r"[+-]?(?:\d+\.\d*|\d*\.\d+)", value):
        return float(value)
    raise TomlError(f"unsupported TOML value {value!r}")


def _fallback_load(text: str) -> dict[str, Any]:
    root: dict[str, Any] = {}
    current = root
    lines = text.splitlines()
    index = 0
    while index < len(lines):
        line_number = index + 1
        line = _strip_comment(lines[index]).strip()
        index += 1
        if not line:
            continue
        if line.startswith("[[") and line.endswith("]]"):
            name = line[2:-2].strip()
            if not re.fullmatch(r"[A-Za-z0-9_-]+", name):
                raise TomlError(f"line {line_number}: invalid array-table name")
            tables = root.setdefault(name, [])
            if not isinstance(tables, list):
                raise TomlError(f"line {line_number}: duplicate TOML key {name!r}")
            current = {}
            tables.append(current)
            continue
        if line.startswith("["):
            raise TomlError(f"line {line_number}: unsupported TOML table")
        if "=" not in line:
            raise TomlError(f"line {line_number}: expected key = value")
        key, value = (part.strip() for part in line.split("=", 1))
        if not re.fullmatch(r"[A-Za-z0-9_-]+", key):
            raise TomlError(f"line {line_number}: invalid TOML key {key!r}")
        while value.startswith("[") and value.count("[") > value.count("]"):
            if index >= len(lines):
                raise TomlError(f"line {line_number}: unterminated TOML array")
            value += "\n" + _strip_comment(lines[index]).strip()
            index += 1
        if key in current:
            raise TomlError(f"line {line_number}: duplicate TOML key {key!r}")
        current[key] = _parse_value(value)
    return root


def loads_toml(text: str) -> dict[str, Any]:
    if tomllib is not None:
        try:
            return tomllib.loads(text)
        except tomllib.TOMLDecodeError as error:
            raise TomlError(str(error)) from error
    return _fallback_load(text)


def load_toml(path: Path) -> dict[str, Any]:
    try:
        return loads_toml(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise TomlError(f"could not read {path}: {error}") from error
