#!/usr/bin/env python3
"""Prints the corpus's survey against the application's, module by module:
  python3 bench/app/compare.py <out dir> [--json corpus-shape.json]
It runs bench/app/survey.py over the nine generated modules with the labels of bench/app/shape.json
and prints corpus/application pairs for the counts the generator is shaped by."""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MODULES = [("shared/core", "core"), ("shared/model", "model"), ("frontend/web", "web"), ("frontend/frontend", "frontend"),
           ("api/server", "server"), ("api/business", "business"), ("api/infra", "infra"), ("api/allocation", "allocation"), ("api/public", "public")]
ROWS = [("files", ("files",)), ("lines", ("lines",)), ("blank lines", ("blank_lines",)), ("case classes", ("case_classes", "count")),
        ("enums", ("enums", "count")), ("enum cases", ("enums", "cases_total")), ("derives", ("derives", "clauses")), ("givens", ("givens", "count")),
        ("extension methods", ("extensions", "methods")), ("opaque types", ("types", "opaque")), ("inline defs", ("inline", "count")),
        ("macro sites", ("macros", "sites")), ("match groups", ("matches", "case_groups")), ("cases", ("matches", "cases")),
        ("fors", ("fors", "count")), ("lambdas", ("lambdas", "arrows")), ("placeholders", ("lambdas", "placeholders")),
        ("traits", ("classes", "traits")), ("plain classes", ("classes", "plain")), ("objects", ("classes", "objects")),
        ("defs", ("members", "defs")), ("vals", ("members", "vals")), ("default args", ("members", "default_args")),
        ("named args", ("members", "named_args")), ("varargs", ("members", "varargs")), ("s-interpolations", ("strings", "s")),
        ("custom interpolations", ("strings", "custom")), ("triple-quoted", ("strings", "triple_quoted")), ("imports", ("imports", "total"))]


def get(module, path):
    value = module
    for key in path:
        value = value.get(key, 0) if isinstance(value, dict) else 0
    return value if isinstance(value, (int, float)) else 0


def main():
    args = sys.argv[1:]
    out = args[0]
    target = args[args.index("--json") + 1] if "--json" in args else os.path.join(out, "corpus-shape.json")
    roots = [os.path.join(out, path) for path, _ in MODULES]
    names = [f"--name={os.path.join(out, path)}:{label}" for path, label in MODULES]
    subprocess.run([sys.executable, os.path.join(HERE, "survey.py"), *roots, *names, "--json", target], check=True, stdout=subprocess.DEVNULL)
    app = json.load(open(os.path.join(HERE, "shape.json")))["modules"]
    corpus = json.load(open(target))["modules"]
    labels = [label for _, label in MODULES]
    print(f"{'corpus/app':22}" + "".join(f"{l[:12]:>14}" for l in labels) + f"{'total':>16}")
    for name, path in ROWS:
        a = [get(app[l], path) for l in labels]
        c = [get(corpus.get(l, {}), path) for l in labels]
        print(f"{name:22}" + "".join(f"{ci:>7}/{ai:<6}" for ai, ci in zip(a, c)) + f"{sum(c):>8}/{sum(a):<7}")


if __name__ == "__main__":
    main()
