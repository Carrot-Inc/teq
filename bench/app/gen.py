#!/usr/bin/env python3
"""Generates the application corpus: python3 bench/app/gen.py <out dir> [--seed N] [--scale F] [--<flag>]...

The corpus is an observatory network's scheduling system in three trees under <out dir>:
`shared/` (the core frameworks of bench/app/src/core and the generated model, api and samples
packages), `frontend/` (the web framework of bench/app/src/web with the class catalog of
bench/macros and the generated frontend) and `api/` (the generated server, business, infra,
allocation and public packages). Sizes and distributions come from bench/app/shape.json, scaled
by --scale. Everything is deterministic from --seed (default 7).

A flag switches a construct teq lacks today into the corpus, where scalac remains the reference:
  --zio-json           codecs from zio-json's jar in the place of the core's json package
  --kittens            derives Eq, Show, Order, Monoid through kittens in the place of the core's
  --cats-data          Several as cats.data.NonEmptyList
  --java-time          java.time in the place of the core's time package
  --big-decimal        BigDecimal amounts in the place of the core's Amount
  --show-interpolator  cats' show interpolator in the pages
  --cats-ordering      cats' Order to Ordering conversion in the place of the core's given
  --inline-fc          `fc:` inline components named by sourcecode's FullName and Line in the pages
  --conversions        a Conversion given over every Put, so the sql interpolator takes any bound value
  --std-members        Product.productElementName and String.map to another element type
--list-flags prints them. Each side gets a project.scala for scala-cli; tests/app.sh builds both.
"""
import json
import os
import shutil
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from genlib import api, frontend, shared  # noqa: E402
from genlib.universe import Flags, Rng, Universe  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))

MODULES = ["core", "model", "web", "frontend", "server", "business", "infra", "allocation", "public"]


def main():
    args = sys.argv[1:]
    if "--list-flags" in args:
        print(" ".join("--" + f for f in Flags.NAMES))
        return
    out = None
    seed = 7
    scale = 1.0
    enabled = []
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--seed":
            seed = int(args[i + 1])
            i += 2
            continue
        if a == "--scale":
            scale = float(args[i + 1])
            i += 2
            continue
        if a.startswith("--seed="):
            seed = int(a.split("=", 1)[1])
        elif a.startswith("--scale="):
            scale = float(a.split("=", 1)[1])
        elif a.startswith("--"):
            enabled.append(a[2:])
        else:
            out = a
        i += 1
    if out is None:
        print(__doc__)
        sys.exit(2)
    flags = Flags(enabled)
    rng = Rng(seed)
    shape = json.load(open(os.path.join(HERE, "shape.json")))["modules"]
    targets = {name: shape[name] for name in MODULES}

    if os.path.exists(out):
        shutil.rmtree(out)
    os.makedirs(out)
    macros = copy_frameworks(out, flags)
    u = Universe(rng, flags)
    endpoints, areas, _ = shared.write_shared(u, out, targets["model"], rng, flags, scale)
    frontend_side = frontend.write_frontend(u, endpoints, areas, out, targets["frontend"], rng, flags, scale, macros)
    api.write_api(u, endpoints, areas, out, targets, rng, flags, scale)
    write_projects(out, flags)
    report(out)
    print(f"frontend: {frontend_side['cls']} cls sites, {frontend_side['tw']} tw sites, {frontend_side['classNames']} classNames sites, {len(endpoints)} endpoints")


def copy_frameworks(out, flags):
    src = os.path.join(HERE, "src")
    shutil.copytree(os.path.join(src, "core"), os.path.join(out, "shared", "core"))
    shutil.copytree(os.path.join(src, "web"), os.path.join(out, "frontend", "web"))
    alt = os.path.join(src, "alt")
    for name, path in [("zio-json", "zio_json/JsonAliases.scala"), ("kittens", "kittens/Derive.scala"), ("cats-data", "cats_data/Several.scala"), ("java-time", "java_time/Time.scala"), ("big-decimal", "big_decimal/Amount.scala")]:
        if getattr(flags, name.replace("-", "_")) and not os.path.exists(os.path.join(alt, path)):
            raise SystemExit(f"--{name}: its alternative framework bench/app/src/alt/{path} is not written yet")
    if flags.zio_json:
        shutil.rmtree(os.path.join(out, "shared", "core", "json"))
        shutil.copy(os.path.join(alt, "zio_json", "JsonAliases.scala"), os.path.join(out, "shared", "core", "JsonAliases.scala"))
    if flags.kittens:
        os.remove(os.path.join(out, "shared", "core", "derive", "Derive.scala"))
        shutil.copy(os.path.join(alt, "kittens", "Derive.scala"), os.path.join(out, "shared", "core", "derive", "Derive.scala"))
    if flags.cats_data:
        os.remove(os.path.join(out, "shared", "core", "collections", "Several.scala"))
        shutil.copy(os.path.join(alt, "cats_data", "Several.scala"), os.path.join(out, "shared", "core", "collections", "Several.scala"))
    if flags.java_time:
        os.remove(os.path.join(out, "shared", "core", "time", "Time.scala"))
        shutil.copy(os.path.join(alt, "java_time", "Time.scala"), os.path.join(out, "shared", "core", "time", "Time.scala"))
    if flags.big_decimal:
        os.remove(os.path.join(out, "shared", "core", "amount", "Amount.scala"))
        shutil.copy(os.path.join(alt, "big_decimal", "Amount.scala"), os.path.join(out, "shared", "core", "amount", "Amount.scala"))
    # The class-name validator of the macro benchmark, re-packaged, with its catalog next to
    # the frontend sources.
    macros_src = open(os.path.join(ROOT, "bench", "macros", "gen.py")).read().replace("\nmain()\n", "\n")
    ns = {}
    exec(compile(macros_src, "bench/macros/gen.py", "exec"), ns)
    validator = ns["VALIDATE_MACROS"].replace("package bench.validate", "package meridian.web.css")
    validator = validator[: validator.index("object Impl:")]
    with open(os.path.join(out, "frontend", "web", "css", "Validator.scala"), "w") as f:
        f.write("// The catalog, its tokens and its checks are those of bench/macros/gen.py.\n" + validator)
    with open(os.path.join(out, "frontend", "classes.txt"), "w") as f:
        f.write(ns["catalog"]())
    return ns


def write_projects(out, flags):
    deps = ["org.typelevel::cats-core::2.13.0", "com.lihaoyi::sourcecode::0.4.2"]
    if flags.zio_json:
        deps.append("dev.zio::zio-json::0.7.1")
    if flags.kittens:
        deps.append("org.typelevel::kittens::3.5.0")
    lines = ["//> using scala 3.8.4", "//> using options -Wconf:any:s -Xmax-inlines 80"] + [f"//> using dep {d}" for d in deps]
    with open(os.path.join(out, "shared", "project.scala"), "w") as f:
        f.write("\n".join(lines) + "\n")


def report(out):
    for side in ("shared", "frontend", "api"):
        files = 0
        lines = 0
        for dirpath, _, names in os.walk(os.path.join(out, side)):
            for n in names:
                if n.endswith(".scala"):
                    files += 1
                    with open(os.path.join(dirpath, n)) as f:
                        lines += sum(1 for _ in f)
        print(f"{side}: {files} files, {lines} lines")


if __name__ == "__main__":
    main()
