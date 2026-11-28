"""Qualified-super witness sources and their JVM member listings."""
from pathlib import Path
import re
import subprocess

SOURCES = Path(__file__).resolve().parent / "witnesses"
MEMBER_CLASSES = {
    "qw_identity": ("identity.B", "identity.T", "identity.C"),
    "qw_default": ("B",),
    "qw_javacases": ("JB", "JD", "SB", "BG"),
    "qw_stack": ("stack.T", "stack.Pin", "stack.C1", "stack.C2", "stack.C3", "stack.C4"),
    "qw_collision": ("B",),
    "qw_value": ("V",),
    "qw_bothmodes": ("bothmodes.Both", "bothmodes.C"),
}


def witness_names():
    return ["qw_" + d.name for d in sorted(SOURCES.iterdir()) if d.is_dir()]


def field(text):
    return text.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n").replace("\r", "\\r")


def unfield(text):
    escapes = {"t": "\t", "n": "\n", "r": "\r", "\\": "\\"}
    return re.sub(r"\\([tnr\\])", lambda m: escapes[m[1]], text)


def member_listing(classes, name):
    lines = []
    for cls in MEMBER_CLASSES[name]:
        result = subprocess.run(["java", "-m", "jdk.jdeps/com.sun.tools.javap.Main",
                                 "-p", "-s", "-classpath", classes, cls],
                                check=True, capture_output=True, text=True, timeout=60)
        lines.append(cls)
        entries = iter(result.stdout.splitlines())
        for line in entries:
            if re.search(r"\s[\w$]*super\$[\w$]*\(", line):
                descriptor = next(entries, "").strip()
                if not descriptor.startswith("descriptor: "):
                    raise ValueError(f"{cls}: missing descriptor for {line}")
                lines.extend((line.strip(), descriptor))
    return "\n".join(lines)
