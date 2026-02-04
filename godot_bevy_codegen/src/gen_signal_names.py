import textwrap
from pathlib import Path
from typing import List

from godot_bevy_codegen.src.gdextension_api import ExtensionApi, GodotClass
from godot_bevy_codegen.src.special_cases import (
    SpecialCases,
    get_type_cfg_attribute,
)
from godot_bevy_codegen.src.util import indent_log, run_cargo_fmt


def generate_signal_names(
    signal_names_file: Path,
    project_root: Path,
    api: ExtensionApi,
) -> None:
    """Generate the signal_names.rs file with signal constants"""
    indent_log("📡 Generating signal names...")

    content = textwrap.dedent("""\
        #![allow(dead_code)]
        //! 🤖 This file is generated. Changes to it will be lost.
        //! To regenerate: uv run python -m godot_bevy_codegen
        //!
        //! Signal name constants for Godot classes.
        //! These provide convenient, discoverable signal names for connecting to Godot signals.
        //!
        //! Example usage:
        //! ```ignore
        //! use godot_bevy::interop::signal_names::ButtonSignals;
        //! // Connect to the "pressed" signal
        //! button.connect(ButtonSignals::PRESSED.into(), callable);
        //! ```
        
        """)

    # Collect all classes with signals, sorted by name, skipping excluded classes
    classes_with_signals: List[GodotClass] = [
        c for c in api.classes if c.signals is not None
    ]

    signal_count = 0

    # Generate a dedicated *Signals struct and impl block for each class
    for godot_class in classes_with_signals:
        rust_class_name = SpecialCases.fix_godot_class_name_for_rust(godot_class.name)
        signals_struct_name = f"{rust_class_name}Signals"

        # Optional: cfg-gate the whole struct/impl if the class is version-gated
        cfg_attr = get_type_cfg_attribute(godot_class.name)
        if cfg_attr:
            content += cfg_attr

        # Struct declaration
        content += f"/// Signal constants for `{rust_class_name}`\n"
        content += f"pub struct {signals_struct_name};\n\n"

        # Impl block
        if cfg_attr:
            content += cfg_attr
        content += f"impl {signals_struct_name} {{\n"

        # Generate constants for each signal
        for signal in godot_class.signals:
            signal_name = signal.name
            description = signal.description.strip()
            const_name = signal_name_to_const(signal_name)

            # Add doc comment with description if available
            if description:
                # Convert BBCode to Markdown
                description = bbcode_to_markdown(description)
                # Sanitize to prevent escaping doc comments
                description = sanitize_doc_comment(description)

                # Format description for Rust doc comments
                description_lines = description.replace("\r\n", "\n").split("\n")
                for line in description_lines:
                    # Strip trailing whitespace but preserve empty lines
                    line = line.rstrip()
                    content += f"    /// {line}\n"
            else:
                # Fallback: just mention the signal name
                content += f"    /// Signal `{signal_name}`\n"

            # Constant definition
            content += (
                f'    pub const {const_name}: &\'static str = "{signal_name}";\n\n'
            )
            signal_count += 1

        # Close impl block
        content += "}\n\n"

    # Write the file
    with open(signal_names_file, "w") as f:
        f.write(content)

    indent_log(
        f"✅ Generated {signal_count} signal constants across {len(classes_with_signals)} classes"
    )
    run_cargo_fmt(signal_names_file, project_root)


def signal_name_to_const(signal_name: str) -> str:
    """Convert a signal name to UPPER_SNAKE_CASE constant name"""
    import re

    # Handle empty or invalid names
    if not signal_name:
        return "SIGNAL"

    # Insert underscores before uppercase letters (for camelCase/PascalCase)
    result = re.sub("([a-z0-9])([A-Z])", r"\1_\2", signal_name)

    # Replace non-alphanumeric characters with underscores
    result = re.sub(r"[^a-zA-Z0-9_]", "_", result)

    # Convert to uppercase
    result = result.upper()

    # Collapse multiple underscores
    result = re.sub(r"_+", "_", result)

    # Strip leading/trailing underscores
    result = result.strip("_")

    # Ensure it doesn't start with a digit (prepend underscore if needed)
    if result and result[0].isdigit():
        result = "_" + result

    # Fallback if empty after processing
    if not result:
        result = "SIGNAL"

    return result


def bbcode_to_markdown(text: str) -> str:
    """Convert Godot BBCode format to Rustdoc-compatible Markdown"""
    import re
    from textwrap import dedent

    # Basic inline formatting
    text = text.replace("[b]", "**").replace("[/b]", "**")
    text = text.replace("[i]", "*").replace("[/i]", "*")
    text = text.replace("[code]", "`").replace("[/code]", "`")

    # [member something] -> `something`
    text = re.sub(r"\[member\s+([^]]+)]", r"`\1`", text)

    # [param something] -> `something`
    text = re.sub(r"\[param\s+([^]]+)]", r"`\1`", text)

    # [constant something] -> `something`
    text = re.sub(r"\[constant\s+([^]]+)]", r"`\1`", text)

    # [method something] -> `something()`
    text = re.sub(r"\[method\s+([^]]+)]", r"`\1()`", text)

    # [signal something] -> `something`
    text = re.sub(r"\[signal\s+([^]]+)]", r"`\1`", text)

    # [enum something] -> `something`
    text = re.sub(r"\[enum\s+([^]]+)]", r"`\1`", text)

    # [url=...]...[/url] -> [link text](url)
    text = re.sub(r"\[url=([^]]+)]([^\[]+)\[/url]", r"[\2](\1)", text)

    # [codeblock]...[/codeblock] -> ```text\n...\n```
    def codeblock_repl(m):
        code = m.group(1).strip()
        # Dedent the code block
        code = dedent(code)
        return f"\n```text\n{code}\n```\n"

    text = re.sub(r"\[codeblock](.*?)\[/codeblock]", codeblock_repl, text, flags=re.S)

    # [codeblocks] (with language specified)
    def codeblocks_repl(m):
        code = m.group(1).strip()
        code = dedent(code)
        return f"\n```gdscript\n{code}\n```\n"

    text = re.sub(
        r"\[codeblocks](.*?)\[/codeblocks]", codeblocks_repl, text, flags=re.S
    )

    # Remove any remaining BBCode-style tags that we didn't handle
    text = re.sub(r"\[/?[a-zA-Z0-9_]+]", "", text)

    return text


def sanitize_doc_comment(text: str) -> str:
    """Sanitize text to be safe for Rustdoc /// comments"""
    # The main concern is preventing */ or */ sequences that could escape the comment
    # Also handle other problematic sequences

    # Replace tabs with 4 spaces for consistent formatting
    text = text.replace("\t", "    ")

    # Replace */ with *\/ to prevent closing block comments
    text = text.replace("*/", r"*\/")

    # Replace leading /// with \/\/\/ to prevent nested doc comments
    text = text.replace("///", r"\/\/\/")

    # Ensure we don't have unclosed backticks that would break markdown
    # Count backticks and add one if odd
    backtick_count = text.count("`")
    if backtick_count % 2 != 0:
        text += "`"

    return text
