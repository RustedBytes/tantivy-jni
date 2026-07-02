#!/usr/bin/env python3
"""Generate a deterministic source-level Kotlin public API snapshot."""

from __future__ import annotations

import argparse
import difflib
import pathlib
import re
import sys

DECLARATION = re.compile(
    r"^\s*(?:(public|internal|private|protected)\s+)?"
    r"(?:(?:data|sealed|open|abstract|final|value|suspend|operator|infix|inline|external)\s+)*"
    r"(class|interface|object|enum class|annotation class|fun|val|var|typealias)\s+(.+)$"
)

CONTAINER_KINDS = ("class", "interface", "object", "enum class", "annotation class")

SKIP_PREFIXES = (
    "companion object",
    "override ",
)


def normalize(line: str) -> str:
    return " ".join(line.strip().split())


def is_public_declaration(line: str) -> bool:
    line = normalize(line)
    if not line or line.startswith("@"):
        return False
    if line.startswith(("private ", "internal ", "protected ")):
        return False
    if any(line.startswith(prefix) for prefix in SKIP_PREFIXES):
        return False
    return DECLARATION.match(line) is not None


def declaration_kind(line: str) -> str | None:
    match = DECLARATION.match(normalize(line))
    return match.group(2) if match else None


def collect_multiline_declaration(lines: list[str], start: int) -> tuple[str, int]:
    parts = [normalize(lines[start])]
    index = start
    paren_balance = parts[0].count("(") - parts[0].count(")")
    while paren_balance > 0 and index + 1 < len(lines):
        index += 1
        next_line = normalize(lines[index]).rstrip("{")
        if next_line:
            parts.append(next_line)
            paren_balance += next_line.count("(") - next_line.count(")")
    return " ".join(parts).rstrip(" {"), index


def direct_public_context(depth: int, public_containers: list[int]) -> bool:
    return depth == 0 or bool(public_containers and depth == public_containers[-1])


def public_api(source: pathlib.Path) -> list[str]:
    entries: list[str] = ["# Tantivy Android public API", ""]
    for file in sorted(source.rglob("*.kt")):
        relative = file.relative_to(source)
        lines = file.read_text(encoding="utf-8").splitlines()
        declarations: list[str] = []
        index = 0
        depth = 0
        public_containers: list[int] = []
        while index < len(lines):
            while public_containers and depth < public_containers[-1]:
                public_containers.pop()

            line = lines[index]
            if direct_public_context(depth, public_containers) and is_public_declaration(line):
                start_index = index
                declaration, index = collect_multiline_declaration(lines, index)
                declarations.append(declaration)
                kind = declaration_kind(declaration)
                raw_declaration = "\n".join(lines[start_index : index + 1])
                if kind in CONTAINER_KINDS and "{" in raw_declaration:
                    public_containers.append(depth + 1)
                depth += raw_declaration.count("{") - raw_declaration.count("}")
            elif direct_public_context(depth, public_containers) and normalize(line).startswith("companion object"):
                public_containers.append(depth + 1)
                depth += line.count("{") - line.count("}")
            else:
                depth += line.count("{") - line.count("}")
            if depth < 0:
                depth = 0
            index += 1
        if declarations:
            entries.append(f"## {relative}")
            entries.extend(f"- {declaration}" for declaration in declarations)
            entries.append("")
    return entries


def write_snapshot(output: pathlib.Path, lines: list[str]) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")


def check_snapshot(expected: pathlib.Path, lines: list[str]) -> int:
    current = "\n".join(lines).rstrip() + "\n"
    if not expected.exists():
        print(f"Missing API snapshot: {expected}", file=sys.stderr)
        return 1
    previous = expected.read_text(encoding="utf-8")
    if previous == current:
        return 0
    diff = difflib.unified_diff(
        previous.splitlines(keepends=True),
        current.splitlines(keepends=True),
        fromfile=str(expected),
        tofile="generated",
    )
    sys.stderr.writelines(diff)
    return 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--check", type=pathlib.Path)
    args = parser.parse_args()

    lines = public_api(args.source)
    if args.output:
        write_snapshot(args.output, lines)
    if args.check:
        return check_snapshot(args.check, lines)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
