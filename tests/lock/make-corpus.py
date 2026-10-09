# Writes corpus.json, the tree of the conformance corpus, and corpus.lock, its canonical text by
# check-export.py's writer; tests/lock.sh then holds every reader and writer to them.
import importlib.util, json, os
here = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("c", os.path.join(here, "../../integrations/sbt/example/check-export.py"))
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)
sha1 = "ff15f2278065734035de325534184128c33eeba1"
reserved = sorted(c.RESERVED)
tree = {
    "teq": "0.1.2",
    "format": 1,
    "binaries": {
        "linux-x86_64": f"https://repo.example/maven2/build/teq/teq/0.1.2/teq-0.1.2-linux-x86_64.exe {sha1} 17334592",
        "osx-aarch_64": f"https://repo.example/a%20b/teq.exe {sha1} 1",
    },
    "jars": {
        "a:b:1": f"maven-central {sha1} 0",
        "a:b:1:jdk8": f"local {sha1} 7 a/b/1/b-1-20261005.101010-3.jar",
        "a:b:1@build": f"local {sha1} 9007199254740991 \"a/b/1@build/b 1.jar\"",
    },
    "keys": {k: k for k in [
        "a", "a.b", "a:b", "a/b", "_a", ".a", "a@b", "a+b", "a-b", "A9", "0a", "9z",
        "2", "17", "1.0", "1e3", ".5", "0x1F", "0o17", "0b101", "2026-10-05", "10:30", "1_000",
        "", " a", "a ", "a b", "-a", "+a", ":a", "a:", "a: b", "a #b", "#a", "@a", "{a}", "[a]", "a,b", "a'b", 'a"b', "a\\b",
        "~", "!a", "&a", "*a", "|a", ">a", "%a", "`a", "?a", "a=b",
        "é", "a\tb", "a\nb", "a\u0085b", "a b", "a b", "😀",
    ] + reserved},
    "values": {
        "plain": ["x", "x.y/z:1@b", "https://repo1.maven.org/maven2/", "org.typelevel:cats-core_3:2.13.0", "3.8.4", "0.1.0-pre.1", "/abs/path", "_x", ".", "..", ".gitignore", "a::b", "1.2.3e4", sha1],
        "quoted": reserved + ["", "2", "-1", "1.0", "1e3", "1E-3", ".5", "007", "0x1F", "0o17", "0b101", "2026-10-05", "2026-10-05T10:00:00Z", "10:30", "1_000", "-Xmx1g", "+1", "a: b", "a #b", "a:", "{a}", "[a]", "a,b", "~"],
        "text": ["a few words", "maven-central " + sha1 + " 12", "a 2 true", "x -y", "2 x", "true x", "a  b", " a", "a ", "a b:"],
        "escapes": ["\"", "\\", "\n", "\r", "\t", "\u0000", "\u0001", "\u001f", "\u007f", "\u0080", "\u0085", "\u009f", " ", " ", "﻿", "￾", "￿", " ", "é", "😀", "a\"b\\c\nd"],
        "integers": [0, 1, -1, 123456789012, 9007199254740991, -9007199254740991],
        "truths": [True, False],
        "empty": [[], {}],
    },
    "nested": {
        "lists": [["a", ["b", 1]], [], {}, {"k": "v", "l": ["x"]}, {"only": {"deep": {"er": True}}}],
        "flow": {"projects": {}},
    },
    "projects": {
        "p": {
            "configurations": {
                "compile": {
                    "classpath": [
                        {"configuration": "compile", "project": "q"},
                        "a:b:1",
                        {"file": "lib/x y.jar"},
                        {"file": "true"},
                        {"a": {"b": {"c": "d"}}, "e": {}},
                        {"k": "a: b", "": "", "on": "on"},
                    ],
                },
            },
            "stage": {
                "layers": {
                    "2": [
                        {"from": {"configuration": "compile", "project": "p"}, "manifest": {"Main-Class": "M", "Implementation-URL": "https://x.example/a?b#c"}, "to": "opt/docker/lib/p.jar"},
                        {"classpath": ["lib/a.jar"], "script": "opt/docker/bin/p"},
                        {"classpath": [], "script": "opt/docker/bin/q"},
                    ],
                },
            },
        },
    },
}
text = c.canonical(tree)
assert c.parse(text) == tree
with open(os.path.join(here, "corpus.lock"), "w", encoding="utf-8", newline="\n") as f:
    f.write(text)
with open(os.path.join(here, "corpus.json"), "w", encoding="utf-8", newline="\n") as f:
    f.write(json.dumps(tree, ensure_ascii=True, indent=1, sort_keys=True) + "\n")
print(len(text.encode()), "bytes", text.count("\n"), "lines")
