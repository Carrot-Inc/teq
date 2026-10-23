#!/usr/bin/env python3
"""SemanticDB's forms against scalac 3.8.4's: the corpus, its golds and the comparison (tests/semanticdb/README.md).

  python3 tests/semanticdb/harness.py --regen [--only GROUP]   scalac 3.8.4 -Xsemanticdb over each group, the
                                                             protobufs decoded into gold/<group>/<document>.json
  python3 tests/semanticdb/harness.py --check                  the manifest and the golds agree: every listed
                                                             document has a gold and every gold is listed
  python3 tests/semanticdb/harness.py --self-test              the comparison refuses damaged documents: a symbol
                                                             removed, a scope reordered, the fake tail permuted,
                                                             an Empty prefix replaced, a document missing, a
                                                             protobuf truncated (on the regenerated output)
  python3 tests/semanticdb/harness.py --teq TEQ [--only GROUP] teq's documents (`TEQ_FIX_SEMANTICDB`, written by
                                                             `teq compiler fix`) over each group, compared with the
                                                             golds field by field; a `pass` row that differs fails

The oracle is scalac 3.8.4's compiler as coursier resolves `org.scala-lang:scala3-compiler_3:3.8.4` (the jars
`scala-cli -S 3.8.4` runs), started as dotty's SemanticdbTests starts it (compiler/test/dotty/tools/dotc/semanticdb/
SemanticdbTests.scala 127-147 at 3.8.4): `dotty.tools.dotc.Main` with `-Xsemanticdb -sourceroot <group root>` in a
JVM of its own, on the system JDK. dotty's own corpus is read from the scala3 checkout ($SCALA3, by default
teq-ref/scala3 beside the main checkout) at the tag 3.8.4, never copied. Each scalac run is bounded (300 s).
Environment: SCALA3, COURSIER (the cs launcher, default `cs`), JAVA (default `java`), JAVAC (default `javac`).
"""

import argparse
import contextlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import semanticdb  # noqa: E402

ROOT = HERE.parent.parent
CORPUS = HERE / "corpus"
GOLD = HERE / "gold"
MANIFEST = HERE / "manifest.txt"
FORMS = HERE / "forms"
# Forms views marked for the rebase: `<group>/<uri>\t<reason>`, the view scalac's forms, not compared until the
# rebase that takes the helpers the reason names (README.md, "Marked for the rebase").
REBASE = HERE / "forms-rebase.txt"
TIMEOUT = 300
COMPILER = "org.scala-lang:scala3-compiler_3:3.8.4"
TAG = "3.8.4"


def scala3_checkout():
    env = os.environ.get("SCALA3")
    if env:
        return Path(env)
    common = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "--git-common-dir"], capture_output=True,
                            text=True, check=True).stdout.strip()
    return (ROOT / common).resolve().parent.parent / "teq-ref" / "scala3"


def fetch(coords):
    cs = os.environ.get("COURSIER", "cs")
    out = subprocess.run([cs, "fetch", "-p", *coords], capture_output=True, text=True, timeout=TIMEOUT)
    if out.returncode != 0:
        raise SystemExit(f"semanticdb: cs fetch {' '.join(coords)} failed:\n{out.stderr}")
    return out.stdout.strip()


def run(cmd, cwd=None):
    out = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=TIMEOUT)
    if out.returncode != 0:
        raise SystemExit(f"semanticdb: {' '.join(map(str, cmd[:3]))} ... failed ({out.returncode}):\n"
                         f"{out.stdout[-4000:]}{out.stderr[-4000:]}")
    return out


def scalac(compiler_cp, root, sources, target, classpath=(), extra=(), cwd=None):
    """scalac 3.8.4 over `sources` (paths under `root`, given relative to `cwd` where one is named), its SemanticDB
    under `target`; `.java` sources among them parsed by dotty's Java parser."""
    target.mkdir(parents=True, exist_ok=True)
    cp = os.pathsep.join([str(target), *map(str, classpath)])
    java = os.environ.get("JAVA", "java")
    given = [x if isinstance(x, str) else os.path.relpath(x, cwd) if cwd else str(x) for x in sources]
    cmd = [java, "-cp", compiler_cp, "dotty.tools.dotc.Main", "-Xsemanticdb", "-d", str(target),
           "-sourceroot", str(root), "-classpath", cp, "-usejavacp", *extra, *given]
    run(cmd, cwd=cwd)


def teq(binary, root, sources, target, classpath=(), extra=(), cwd=None):
    """teq over `sources`, its documents under `target` as scalac's: `teq compiler fix` with `TEQ_FIX_SEMANTICDB` set
    and `TEQ_FIX_SEMANTICDB_ROOT` the group's root (scalac's `-sourceroot`), the sources given relative to `cwd` (the
    root where none is named), a semantic rule run in `--check` mode (its findings not read; a program that does not
    type writes no document, which the collection then names). Its status is returned, `None` past the bound: 0, 1
    for a rule's finding, 2 for a program that does not type (src/fix/engine.rs's `round`), which a refusal must
    explain; any other is the run's failure (`teq_group`). The sources are given as
    scalac's are, a `.java` one too, which teq does not read: the group of the Java class from source compares
    teq's answer for it with scalac's. scalac's options (`extra`) are not teq's: the documents do not depend on them.
    The program is typed under scala-library, the library scalac's documents name (README.md, "Provenance")."""
    target.mkdir(parents=True, exist_ok=True)
    # The standard library is teq's own scala-library 3.8.4, as scalac's is its own: a dependency's older
    # scala-library or scala3-library jar on the class path would be taken as the std instead.
    cp = [target, *(c for c in classpath if not re.match(r"scala3?-library(_3)?-[0-9]", Path(c).name))]
    cwd = cwd or root
    cmd = [str(binary), "compiler", "fix", *(x if isinstance(x, str) else os.path.relpath(x, cwd) for x in sources),
           "--rules", "NoUnusedImports", "--check", "--std=scala-library"]
    if cp:
        cmd += ["--classpath", os.pathsep.join(map(str, cp))]
    env = dict(os.environ, TEQ_FIX_SEMANTICDB=str(target), TEQ_FIX_SEMANTICDB_ROOT=str(root))
    try:
        out = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        (target.parent / "teq.log").write_text(f"semanticdb: teq ran past {TIMEOUT}s\n", encoding="utf-8")
        return None
    (target.parent / "teq.log").write_text(out.stdout + out.stderr, encoding="utf-8")
    return out.returncode


def scala_sources(d):
    return sorted(p for p in d.rglob("*.scala"))


# Each group: how its sources are laid out and compiled. `docs` are the documents the group must give, as
# sourceroot-relative paths; a missing or an extra document is an error.
def group_dotty(work, compiler_cp, compile):
    s3 = scala3_checkout()
    if not (s3 / ".git").exists() and not (s3 / "HEAD").exists():
        raise SystemExit(f"semanticdb: no scala3 checkout at {s3} (set SCALA3)")
    names = subprocess.run(["git", "-C", str(s3), "ls-tree", "-r", "-z", "--name-only", TAG, "tests/semanticdb"],
                           capture_output=True, text=True, check=True).stdout.split("\0")
    src = work / "src"
    for n in names:
        if n.startswith("tests/semanticdb/expect/") or n.startswith("tests/semanticdb/javacp/"):
            dest = src / n[len("tests/semanticdb/"):]
            dest.parent.mkdir(parents=True, exist_ok=True)
            blob = subprocess.run(["git", "-C", str(s3), "show", f"{TAG}:{n}"], capture_output=True, check=True)
            dest.write_bytes(blob.stdout)
    expect = src / "expect"
    inputs = [p for p in scala_sources(expect) if not p.name.endswith(".expect.scala")]
    target = work / "out"
    target.mkdir(parents=True)
    javac = os.environ.get("JAVAC", "javac")
    run([javac, "-d", str(target), *map(str, sorted((src / "javacp").rglob("*.java")))])
    # SemanticdbTests.scala 133-146 at 3.8.4, its options as they are.
    compile(expect, inputs, target, extra=["-feature", "-deprecation", "-Xignore-scala2-macros",
           "-Wunused:all", "-Yreporter:dotty.tools.dotc.reporting.Reporter$SilentReporter"])
    return target, expect, inputs


def group_dir(name, extra_cp=()):
    def g(work, compiler_cp, compile):
        root = CORPUS / name
        target = work / "out"
        inputs = scala_sources(root)
        compile(root, inputs, target, classpath=extra_cp() if callable(extra_cp) else extra_cp)
        return target, root, inputs
    return g


def group_lib_source(work, compiler_cp, compile):
    # The dependency and its consumer compiled together: the consumer's document sees source symbols.
    root = CORPUS
    inputs = scala_sources(CORPUS / "lib") + scala_sources(CORPUS / "client")
    target = work / "out"
    compile(root, inputs, target)
    return target, root, inputs


def group_lib_jar(work, compiler_cp, compile):
    # The dependency compiled first; the consumer against its classes alone (TASTy): the consumer's document sees
    # loaded symbols, the companion `CC`'s synthetic `apply` before its written one in the pickled scope.
    lib = work / "lib"
    scalac(compiler_cp, CORPUS, scala_sources(CORPUS / "lib"), lib)
    shutil.rmtree(lib / "META-INF")
    target = work / "out"
    inputs = scala_sources(CORPUS / "client")
    compile(CORPUS, inputs, target, classpath=[lib])
    return target, CORPUS, inputs


def library_groups(lib, client):
    """A library and its consumer in three forms: compiled together; the consumer against scalac's class directory of
    the library (TASTy, the annotation read from the pickle by `TreeUnpickler.readType`); and against a jar of those
    classes. The consumer's documents equal across the three in scalac's golds."""
    def source(work, compiler_cp, compile):
        inputs = scala_sources(CORPUS / lib) + scala_sources(CORPUS / client)
        target = work / "out"
        compile(CORPUS, inputs, target)
        return target, CORPUS, inputs

    def lib_classes(work, compiler_cp):
        out = work / "lib"
        scalac(compiler_cp, CORPUS, scala_sources(CORPUS / lib), out)
        shutil.rmtree(out / "META-INF")
        return out

    def classes(work, compiler_cp, compile):
        out = lib_classes(work, compiler_cp)
        target = work / "out"
        inputs = scala_sources(CORPUS / client)
        compile(CORPUS, inputs, target, classpath=[out])
        return target, CORPUS, inputs

    def jar(work, compiler_cp, compile):
        import zipfile
        out = lib_classes(work, compiler_cp)
        packed = work / "lib.jar"
        with zipfile.ZipFile(packed, "w") as z:
            for f in sorted(out.rglob("*")):
                if f.is_file():
                    z.write(f, f.relative_to(out).as_posix())
        target = work / "out"
        inputs = scala_sources(CORPUS / client)
        compile(CORPUS, inputs, target, classpath=[packed])
        return target, CORPUS, inputs

    return source, classes, jar


# A library whose signature carries an annotation (`Box[Int @Ann]`), the pass over c0dbae640's.
group_annotated_source, group_annotated_classes, group_annotated_jar = library_groups("annotated-lib", "annotated-client")
# The annotation's class an inner class selected through a path (`Box[Int @outer.Ann]`) and through the enclosing
# class's `this` (`Box[Int @Ann]` inside `Outer`), the pass over 8b8caaa92's: `Annotations.annotClass`.
group_nested_source, group_nested_classes, group_nested_jar = library_groups("nested-lib", "nested-client")
# A generic inner class selected through a path as an annotation (`Box[Int @outer.Ann[String]]`), the pass over
# 32fc738c3's: pending for type-representation, where the class reference is one `TypeRef` (teq's `Lambda` over a
# `Nested` stands for it here, and the class extraction does not read it).
group_generic_source, group_generic_classes, group_generic_jar = library_groups("generic-lib", "generic-client")


def http4s_classpath():
    return fetch(["org.http4s:http4s-core_3:0.23.32"]).split(os.pathsep)


def group_java_order_source(work, compiler_cp, compile):
    # A Java class compiled with its consumer: dotty parses the Java source and enters its members in written order
    # (the method `size()` before the field `size`), so the method is `size().`.
    root = CORPUS / "java-order"
    target = work / "out"
    compile(root, sorted(root.rglob("*.java")) + scala_sources(root), target)
    return target, root, scala_sources(root)


def group_java_order_classes(work, compiler_cp, compile):
    # The same class compiled by javac and loaded from its class file: ClassfileParser enters fields before methods
    # (ClassfileParser.scala 407-408 at 3.8.4), so the field comes first and the method is `size(+1).`.
    root = CORPUS / "java-order"
    classes = work / "classes"
    classes.mkdir()
    run([os.environ.get("JAVAC", "javac"), "-d", str(classes), *map(str, sorted(root.rglob("*.java")))])
    target = work / "out"
    compile(root, scala_sources(root), target, classpath=[classes])
    return target, root, scala_sources(root)


def group_outside_root(work, compiler_cp, compile):
    # A source outside the sourceroot: dotty names its document by its path as given (SourceFile.relativePath's
    # fallback), here relative to the group's directory, from which both are given.
    base = CORPUS / "outside-root"
    root = base / "root"
    target = work / "out"
    inputs = [root / "In.scala", base / "outside" / "Other.scala"]
    compile(root, inputs, target, cwd=base)
    return target, root, inputs, base


def group_dot_path(work, compiler_cp, compile):
    # A source outside the sourceroot given with an interior `.`: dotty names its document by the path's elements as
    # Java's path iterator gives them (`Tools.mkURIstring`), the `.` kept: `outside/./Other.scala`.
    base = CORPUS / "dot-path"
    root = base / "inside"
    target = work / "out"
    inputs = ["outside/./Other.scala"]
    compile(root, inputs, target, cwd=base)
    return target, root, inputs, base


GROUPS = {
    "dotty": group_dotty,
    "design": group_dir("design"),
    "lib-source": group_lib_source,
    "lib-jar": group_lib_jar,
    "traversal/p": group_dir("traversal/p"),
    "traversal/q": group_dir("traversal/q"),
    "traversal/r": group_dir("traversal/r"),
    "traversal/s": group_dir("traversal/s"),
    "fake-order": group_dir("fake-order"),
    "fun-param/v0": group_dir("fun-param/v0"),
    "fun-param/v1": group_dir("fun-param/v1"),
    "fun-param/v2": group_dir("fun-param/v2"),
    "fun-param/v3": group_dir("fun-param/v3"),
    "numbers": group_dir("numbers"),
    "unicode": group_dir("unicode"),
    "java-order-source": group_java_order_source,
    "outside-root": group_outside_root,
    "dot-path": group_dot_path,
    "retained-maps": group_dir("retained-maps"),
    "ranges": group_dir("ranges"),
    "given": group_dir("given"),
    "bottom": group_dir("bottom"),
    "annotation-args": group_dir("annotation-args"),
    "annotation-deps": group_dir("annotation-deps"),
    "alias-variance": group_dir("alias-variance"),
    "aliases": group_dir("aliases"),
    "annotation-trees": group_dir("annotation-trees"),
    "simplify-normalize": group_dir("simplify-normalize"),
    "simplify-unchecked": group_dir("simplify-unchecked"),
    "simplify-meet": group_dir("simplify-meet"),
    "nested-annotations": group_dir("nested-annotations"),
    "bounds": group_dir("bounds"),
    "range-departures": group_dir("range-departures"),
    "bound-order": group_dir("bound-order"),
    "alias-bound": group_dir("alias-bound"),
    "module-singleton": group_dir("module-singleton"),
    "annotated-source": group_annotated_source,
    "annotated-classes": group_annotated_classes,
    "annotated-jar": group_annotated_jar,
    "nested-annotated-source": group_nested_source,
    "nested-annotated-classes": group_nested_classes,
    "nested-annotated-jar": group_nested_jar,
    "generic-annotated-source": group_generic_source,
    "generic-annotated-classes": group_generic_classes,
    "generic-annotated-jar": group_generic_jar,
    "java-order-classes": group_java_order_classes,
    "rules/xuwei": group_dir("rules/xuwei"),
    "rules/forbidden": group_dir("rules/forbidden"),
    "rules/typelevel": group_dir("rules/typelevel", http4s_classpath),
}


def dump(doc):
    """A gold's text: the document's scalar fields on their own lines, each element of a list on one line."""
    lines = ["{"]
    keys = list(doc)
    for n, k in enumerate(keys):
        v = doc[k]
        comma = "," if n + 1 < len(keys) else ""
        if isinstance(v, list) and v:
            lines.append(f"  {json.dumps(k)}: [")
            for m, e in enumerate(v):
                lines.append("    " + json.dumps(e, ensure_ascii=False, separators=(",", ":")) +
                             ("," if m + 1 < len(v) else ""))
            lines.append("  ]" + comma)
        else:
            lines.append(f"  {json.dumps(k)}: {json.dumps(v, ensure_ascii=False)}{comma}")
    lines.append("}")
    return "\n".join(lines) + "\n"


# java.net.URI's legal path characters below 0x80 (URI.java's L_PATH: unreserved, escaped, ":@&=+$," and ";/").
URI_SAFE = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.!~*'():@&=+$,;")


def uri_of(rel):
    """A sourceroot-relative path as dotty's Tools.mkURIstring writes it (Tools.scala 12-18 at 3.8.4): each part
    through `new java.net.URI(null, null, "/" + part, null)`, whose `quote` (URI.java) escapes, as the UTF-8 bytes of
    the character, an ASCII character that is no legal path character (a space, `"`, `#`, `%`, `<`, `>`, `?`, `[`,
    `\\`, `]`, `^`, `` ` ``, `{`, `|`, `}`, a control) and, above 0x7F, only `Character.isSpaceChar` (the categories
    Zs, Zl, Zp) and `isISOControl` (0x80-0x9F); every other non-ASCII character is kept as it is (a zero-width
    joiner, a letter, a format character)."""
    import unicodedata

    def part(p):
        out = []
        for ch in p:
            o = ord(ch)
            if o < 0x80:
                keep = ch in URI_SAFE
            else:
                keep = not (unicodedata.category(ch) in ("Zs", "Zl", "Zp") or 0x80 <= o <= 0x9F)
            out.append(ch if keep else "".join("%%%02X" % b for b in ch.encode("utf-8")))
        return "".join(out)
    return "/".join(part(p) for p in rel.split("/"))


def views(target):
    """teq's views of the retained forms (`<file>.forms` beside each document, `TEQ_FIX_SEMANTICDB`), by uri."""
    base = target / "META-INF" / "semanticdb"
    return {uri_of(f.relative_to(base).as_posix()[:-len(".forms")]): f.read_text(encoding="utf-8") for f in sorted(base.rglob("*.forms"))} if base.exists() else {}


def laid_out(group):
    """A group's run as its function returns it: the target, the root, the inputs and the directory the compiler ran
    in (`None`: wherever the harness runs, the inputs given absolute)."""
    return (*group, None) if len(group) == 3 else group


def given_uri(p, root, cwd):
    """A document's uri as dotty names it: the input's path relative to the root, normalized, where it is under the
    root, else its path as given (relative to `cwd`, the directory the compiler ran in), with Java's path elements: an
    input given as a string keeps an interior `.` that a `Path` drops (`Tools.mkURIstring` over `Path.iterator`)."""
    if isinstance(p, str):
        full = os.path.normpath(os.path.join(cwd, p))
        if full.startswith(str(root) + os.sep):
            return uri_of(Path(os.path.relpath(full, root)).as_posix())
        return uri_of("/".join(x for x in p.split("/") if x))
    try:
        return uri_of(p.relative_to(root).as_posix())
    except ValueError:
        return uri_of(Path(os.path.relpath(p, cwd)).as_posix())


def collected(target, root, inputs, cwd=None):
    """The decoded documents under `target` by sourceroot-relative uri, what is wrong with the output itself (a
    document whose protobuf still decodes one byte short, two documents for one uri, a document no input names), and
    the inputs' uris with no document."""
    found, wrong = {}, []
    for f in sorted((target / "META-INF" / "semanticdb").rglob("*.semanticdb")):
        data = f.read_bytes()
        try:
            semanticdb.decode_documents(data[:-1])
        except semanticdb.DecodeError:
            pass
        else:
            wrong.append(f"{f.name}: a truncated protobuf decoded")
            continue
        try:
            docs = semanticdb.decode_documents(data)
        except semanticdb.DecodeError as e:
            wrong.append(f"{f.name}: {e}")
            continue
        for d in docs:
            if d["uri"] in found:
                wrong.append(f"two documents for {d['uri']}")
            found[d["uri"]] = d
    want = {given_uri(p, root, cwd) for p in inputs}
    wrong += [f"a document no input names: {u}" for u in sorted(set(found) - want)]
    return found, wrong, sorted(want - set(found))


def collect(target, root, inputs, cwd=None):
    """The decoded documents under `target`, by sourceroot-relative uri; every input must have one, no other."""
    found, wrong, missing = collected(target, root, inputs, cwd)
    if wrong or missing:
        raise SystemExit(f"semanticdb: {'; '.join(wrong) or 'the output whole'}; documents missing {missing}")
    return found


def regen(groups):
    compiler_cp = fetch([COMPILER])
    for g in groups:
        with tempfile.TemporaryDirectory(prefix="teq-semanticdb-") as tmp:
            target, root, inputs, cwd = laid_out(GROUPS[g](Path(tmp), compiler_cp, lambda *a, **k: scalac(compiler_cp, *a, **k)))
            docs = collect(target, root, inputs, cwd)
        out = GOLD / g
        if out.exists():
            shutil.rmtree(out)
        for uri, d in docs.items():
            p = out / (uri + ".json")
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(dump(d), encoding="utf-8")
        print(f"{g}: {len(docs)} documents")


def read_manifest():
    rows = []
    for line in MANIFEST.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        cols = line.split("\t")
        if len(cols) < 2:
            raise SystemExit(f"semanticdb: manifest row without a status: {line}")
        rows.append((cols[0], cols[1], cols[2] if len(cols) > 2 else ""))
    return rows


def read_rebase():
    """The forms views marked for the rebase, by `<group>/<uri>`, each with its reason."""
    marks = {}
    for line in REBASE.read_text(encoding="utf-8").splitlines() if REBASE.exists() else []:
        if not line.strip() or line.startswith("#"):
            continue
        cols = line.split("\t")
        if len(cols) != 2 or not cols[1].strip():
            raise SystemExit(f"semanticdb: a rebase mark without its reason: {line}")
        marks[cols[0]] = cols[1]
    return marks


def gold_docs():
    """Every gold by its manifest name, `<group>/<uri>`: the uri the document records, its file named after it (an
    interior `.` element of which a file system path drops)."""
    out = {}
    for p in sorted(GOLD.rglob("*.json")):
        rel = p.relative_to(GOLD).as_posix()
        group = max((k for k in GROUPS if rel.startswith(k + "/")), key=len)
        doc = json.loads(p.read_text(encoding="utf-8"))
        out[f"{group}/{doc['uri']}"] = doc
    return out


def golds():
    return set(gold_docs())


def split_gold(g):
    """A gold's group (the longest GROUPS key it starts with) and its document's uri."""
    group = max((k for k in GROUPS if g.startswith(k + "/")), key=len)
    return group, g[len(group) + 1:]


def compare_group(gold, got):
    """The differences between two groups' documents (uri -> document): a missing or an extra document is one."""
    out = []
    for uri in sorted(set(gold) | set(got)):
        if uri not in got:
            out.append((uri, "missing", None))
        elif uri not in gold:
            out.append((uri, "unexpected", None))
        else:
            out.extend((uri, *d) for d in semanticdb.diff(gold[uri], got[uri]))
    return out


def check():
    rows = read_manifest()
    listed = {r[0] for r in rows}
    have = golds()
    bad = False
    for r in sorted(listed - have):
        print(f"listed without a gold: {r}")
        bad = True
    for r in sorted(have - listed):
        print(f"gold not listed: {r}")
        bad = True
    for name, status, _ in rows:
        if status not in ("pending", "pass") and not status.startswith("finding:"):
            print(f"{name}: unknown status {status}")
            bad = True
    groups = {r.split("/")[0] for r in listed}
    print(f"manifest: {len(rows)} documents in {len(groups)} groups")
    marks = read_rebase()
    for name in sorted(marks):
        if not (FORMS / (name + ".forms")).exists():
            print(f"{name}: marked for the rebase without a forms view")
            bad = True
    if marks:
        print(f"forms views marked for the rebase: {len(marks)}")
    return 1 if bad else 0


def compared(doc, unstable=()):
    """What the comparison reads of a document: all but `diagnostics`, the AppendDiagnostics phase's (README.md), the
    symbols of the occurrences at the `unstable` positions (`(line, character)` of their ranges' starts) masked."""
    out = {k: v for k, v in doc.items() if k != "diagnostics"}
    if unstable:
        out["occurrences"] = [dict(o, symbol="<unstable>") if o.get("range") and (o["range"]["start_line"], o["range"]["start_character"]) in unstable else o for o in doc["occurrences"]]
    return out


def unstable_of(note):
    """The positions a manifest note names unstable (`unstable occurrences 5:22 6:6: ...`), none for another note."""
    m = re.match(r"unstable occurrences ((?:\d+:\d+ ?)+):", note or "")
    return frozenset(tuple(int(x) for x in p.split(":")) for p in m.group(1).split()) if m else frozenset()


# The fields a pending document leaves unfinished until the forms are built (order of work, step 7): its body. Every
# other field the comparison reads (the schema, the uri, the text, the MD5 of the source, the language: the document's
# identity) is required as scalac's of every document teq writes.
BODY = ("symbols", "occurrences", "synthetics")


def header_diffs(diffs):
    """The differences outside a document's body."""
    return [d for d in diffs if re.match(r"\$\.([^.\[]+)", d[0]).group(1) not in BODY]


def compare_teq(binary, groups):
    """teq's documents over each group against the golds. A group whose teq run or collection fails fails the run
    (`teq_group`). Each document is equal or differs (its first differences printed): a `pass` row that differs fails
    the run, and a `pending` or `finding:` row whose header (all but `BODY`) differs from scalac's fails it too; a
    document refused fails it unless its row pins the refusal (`pinned_refusal`); an `unstable` row's named
    occurrences are compared against nothing; a `pending` or `finding:` row whose document is now equal is named for
    the manifest to say `pass`."""
    rows = {name: status for name, status, _ in read_manifest()}
    unstable = {name: unstable_of(note) for name, _, note in read_manifest()}
    marks = read_rebase()
    compiler_cp = fetch([COMPILER])
    by_group = {}
    for n, doc in gold_docs().items():
        group, uri = split_gold(n)
        by_group.setdefault(group, {})[uri] = doc
    failed = False
    for g in groups:
        gold = by_group.get(g, {})
        got, refused, seen, error = teq_group(binary, g, compiler_cp)
        if error:
            failed = True
            print(f"  {g}: FAILS: {error}")
            print(f"{g}: {len(gold)} documents, the run failed")
            continue
        # The retained forms teq's typer keeps, against the views read off scalac's typed tree (forms/<group>): checked
        # whatever the document's status, the forms being the store's before the documents read them.
        for f in sorted((FORMS / g).rglob("*.forms")) if (FORMS / g).exists() else []:
            uri = f.relative_to(FORMS / g).as_posix()[:-len(".forms")]
            want, have = f.read_text(encoding="utf-8"), seen.get(uri)
            # A view marked for the rebase is scalac's forms, which the helpers its mark names give: not compared
            # until then, named either way, and its mark to be dropped once teq's forms are scalac's.
            mark = marks.get(f"{g}/{uri}")
            if mark is not None:
                state = "equal: drop its mark" if have == want else "not compared"
                print(f"  {g}/{uri}: forms marked for the rebase, {state} ({mark})")
                continue
            if have != want:
                failed = True
                print(f"  {g}/{uri}: the retained forms differ from scalac's typed tree:\n    want {want!r}\n    have {have!r}")
        equal = 0
        for uri in sorted(gold):
            name = f"{g}/{uri}"
            status = rows.get(name, "unlisted")
            if uri in refused:
                if pinned_refusal(status, refused[uri]):
                    print(f"  {name}: refused by teq ({status}): {refused[uri]}")
                else:
                    failed = True
                    print(f"  {name}: REFUSED by teq, which its row does not pin ({status}): {refused[uri]}")
                continue
            if uri not in got:
                failed = True
                print(f"  {name}: MISSING, teq wrote no document ({status})")
                continue
            u = unstable.get(name, frozenset())
            diffs = semanticdb.diff(compared(gold[uri], u), compared(got[uri], u))
            if not diffs:
                equal += 1
                if status != "pass":
                    print(f"  {name}: equal, the manifest says {status}: mark it pass")
                continue
            first = "; ".join(f"{p}: {json.dumps(x, ensure_ascii=False)[:120]} vs {json.dumps(y, ensure_ascii=False)[:120]}" for p, x, y in diffs[:2])
            head = header_diffs(diffs)
            if status == "pass":
                failed = True
                print(f"  {name}: DIFFERS ({len(diffs)}), the manifest says pass: {first}")
            elif head:
                failed = True
                print(f"  {name}: its HEADER DIFFERS from scalac's ({status}): " + "; ".join(f"{p}: {json.dumps(x, ensure_ascii=False)[:120]} vs {json.dumps(y, ensure_ascii=False)[:120]}" for p, x, y in head[:3]))
            elif os.environ.get("VERBOSE"):
                print(f"  {name}: differs ({len(diffs)}, {status}): {first}")
        gone = sum(1 for uri in gold if uri in refused)
        print(f"{g}: {len(gold)} documents, {equal} equal, {len(gold) - equal - gone} differ, {gone} refused")
    return 1 if failed else 0


def teq_group(binary, g, compiler_cp):
    """teq's documents of a group, the files teq refuses with their first error, its retained forms' views, and the
    run's failure (`None` when there is none).

    teq's fix writes a document for every file of a program that types and none for one that does not (status 2,
    src/fix/engine.rs's `round`). So a run that does not type proves only its refusals: each file it was given that its
    log names with an error is refused (a Java source among them, which has no document of its own), and the rest are
    typed again without it (at most four runs, a file whose absence breaks another refused in turn), that run to write
    every other file's document. A refusal accounts for its own file's document and nothing else; whether its row
    pins it is `compare_teq`'s. Every other fault fails the group at once, with no run after it:
    - teq's run killed, past its bound, or ending with a status other than 0, 1 and 2;
    - a document written by a run that did not type;
    - the output itself wrong: a damaged document, two for one uri, one no input names;
    - a document missing from a run that typed;
    - a run that did not type naming no file it was given with an error."""
    refused = {}
    for _ in range(4):
        with tempfile.TemporaryDirectory(prefix="teq-semanticdb-") as tmp:
            statuses, sources_given = [], set()
            def compile(root, sources, target, cwd=None, **k):
                kept = [x for x in sources if given_uri(x, root, cwd or root) not in refused]
                # With every Scala source refused there is nothing of teq's to type (it reads no Java source).
                if any(str(x).endswith(".scala") for x in kept):
                    sources_given.update(given_uri(x, root, cwd or root) for x in kept)
                    statuses.append(teq(binary, root, kept, target, cwd=cwd, **k))
            target, root, inputs, cwd = laid_out(GROUPS[g](Path(tmp), compiler_cp, compile))
            cwd = cwd or root
            log = (Path(tmp) / "teq.log").read_text(encoding="utf-8") if (Path(tmp) / "teq.log").exists() else ""
            def failed(why):
                return {}, refused, {}, f"{why}\n{log[-2000:]}"
            ran = [c for c in statuses if c not in (0, 1, 2)]
            if ran:
                return failed(f"teq's run failed ({'past its bound' if ran[0] is None else f'status {ran[0]}'})")
            inputs = [x for x in inputs if given_uri(x, root, cwd) not in refused]
            found, wrong, missing = collected(target, root, inputs, cwd)
            if 2 in statuses and found:
                return failed(f"teq's run did not type (status 2) and wrote documents: {sorted(found)}")
            if wrong:
                return failed(f"teq's output is wrong: {'; '.join(wrong)}")
            if 2 not in statuses:
                if missing:
                    return failed(f"documents missing from a run that typed, no refusal naming them: {missing}")
                return found, refused, views(target), None
            new = {}
            for line in log.splitlines():
                # A file of the group, Scala or Java, teq names by its path as given; its own std's are `<std>/`.
                m = re.match(r"([^<].*?\.(?:scala|java)):\d+:\d+: error: (.*)", line)
                if m:
                    u = given_uri(Path(cwd) / m.group(1), root, cwd)
                    if u in sources_given:
                        new.setdefault(u, line)
            if not new:
                return failed(f"teq's run did not type (status 2) and names no file it was given with an error")
            refused.update(new)
    return {}, refused, {}, f"the refusals did not settle in four runs: {sorted(refused)}"


def pinned_refusal(status, error):
    """Whether a refusal is pinned by name: its document's manifest row a `finding:` that says teq refuses it, at the
    place the note names when it names one (`<file>:<line>:<column>`, teq's first error's)."""
    if not status.startswith("finding:") or not re.search(r"\brefuses?\b", status):
        return False
    places = re.findall(r"([\w.$-]+\.(?:scala|java):\d+:\d+)", status)
    return not places or any(f"{pl}: error:" in error for pl in places)


def mutations(doc):
    """Damaged copies of a document, each of which a comparison must refuse."""
    out = []
    if doc["symbols"]:
        d = json.loads(json.dumps(doc))
        d["symbols"].pop(len(d["symbols"]) // 2)
        out.append(("a symbol removed", d))
    if len(doc["symbols"]) > 1:
        d = json.loads(json.dumps(doc))
        d["symbols"][0], d["symbols"][-1] = d["symbols"][-1], d["symbols"][0]
        out.append(("the symbols' order changed (the fake tail permuted)", d))
    if doc["occurrences"]:
        d = json.loads(json.dumps(doc))
        d["occurrences"][0]["range"]["end_character"] += 1
        out.append(("an occurrence's range moved", d))

    def scopes(v):
        if isinstance(v, dict):
            for k, x in v.items():
                if k in ("type_parameters", "declarations", "parameters") and isinstance(x, dict) and \
                        len(x.get("symlinks", [])) > 1:
                    yield x
                yield from scopes(x)
        elif isinstance(v, list):
            for x in v:
                yield from scopes(x)

    d = json.loads(json.dumps(doc))
    sc = next(scopes(d["symbols"]), None)
    if sc is not None:
        sc["symlinks"].reverse()
        out.append(("a scope reordered", d))

    def prefixes(v):
        if isinstance(v, dict):
            if v.get("kind") == "TypeRef" and v["prefix"] == {"kind": "Empty"}:
                yield v
            for x in v.values():
                yield from prefixes(x)
        elif isinstance(v, list):
            for x in v:
                yield from prefixes(x)

    d = json.loads(json.dumps(doc))
    tr = next(prefixes(d["symbols"]), None)
    if tr is not None:
        tr["prefix"] = {"kind": "ThisType", "symbol": "_root_/"}
        out.append(("an Empty prefix replaced", d))

    def constants(v):
        if isinstance(v, dict):
            if v.get("kind") in ("FloatConstant", "DoubleConstant"):
                yield v
            for x in v.values():
                yield from constants(x)
        elif isinstance(v, list):
            for x in v:
                yield from constants(x)

    for c in constants(doc["symbols"]):
        if c["value"].startswith("NaN"):
            d = json.loads(json.dumps(doc))
            next(x for x in constants(d["symbols"]) if x["value"] == c["value"])["value"] = "0.0"
            out.append(("a NaN constant made finite", d))
            break
    return out


def self_test():
    every = gold_docs()
    have = sorted(every)
    if not have:
        raise SystemExit("semanticdb: no golds (run --regen)")
    refused = tried = 0
    for g in have:
        doc = every[g]
        if semanticdb.diff(doc, doc):
            raise SystemExit(f"semanticdb: {g} differs from itself")
        for what, bad in mutations(doc):
            tried += 1
            if semanticdb.diff(doc, bad):
                refused += 1
            else:
                print(f"{g}: {what}: not refused")
    # A document with unstable occurrences: their symbols alone are masked; each damage above is still refused, a
    # changed MD5, another occurrence's symbol and an unstable occurrence's range too, its symbol changed accepted.
    notes = {name: note for name, _, note in read_manifest()}
    for g in have:
        u = unstable_of(notes.get(g))
        if not u:
            continue
        doc = every[g]
        at = [i for i, o in enumerate(doc["occurrences"]) if o.get("range") and (o["range"]["start_line"], o["range"]["start_character"]) in u]
        other = next(i for i, o in enumerate(doc["occurrences"]) if i not in at)
        if len(at) != len(u):
            raise SystemExit(f"semanticdb: {g}: the manifest's unstable occurrences are not all the gold's")

        def changed(path, value, d=doc):
            c = json.loads(json.dumps(d))
            x = c
            for k in path[:-1]:
                x = x[k]
            x[path[-1]] = value
            return c
        damaged = [(w, b) for w, b in mutations(doc)]
        damaged += [("the MD5 changed", changed(["md5"], "0" * 32)),
                    ("another occurrence's symbol changed", changed(["occurrences", other, "symbol"], "x/Y#")),
                    ("an unstable occurrence's range moved", changed(["occurrences", at[0], "range", "end_character"], 99))]
        for what, bad in damaged:
            tried += 1
            if semanticdb.diff(compared(doc, u), compared(bad, u)):
                refused += 1
            else:
                print(f"{g}: {what}: not refused with its unstable occurrences masked")
        tried += 1
        if semanticdb.diff(compared(doc, u), compared(changed(["occurrences", at[0], "symbol"], "local99"), u)):
            print(f"{g}: an unstable occurrence's symbol changed: refused though masked")
        else:
            refused += 1
    # A group with a document missing, or one more, must be refused by the group comparison.
    groups = {}
    for g in have:
        group, uri = split_gold(g)
        groups.setdefault(group, {})[uri] = every[g]
    for group, docs in groups.items():
        uris = sorted(docs)
        tried += 2
        fewer = {u: docs[u] for u in uris[1:]}
        more = dict(docs, **{"<added>/" + uris[0]: docs[uris[0]]})
        for what, other in (("a document removed", fewer), ("a document added", more)):
            if compare_group(docs, other):
                refused += 1
            else:
                print(f"{group}: {what}: not refused")
    print(f"self-test: {refused}/{tried} damaged documents and groups refused over {len(have)} golds")
    return 0 if refused == tried else 1


# A whole document of its own (schema 4, the uri `unexpected.scala`, language SCALA) in a TextDocuments message.
UNEXPECTED = b"\x0a\x16\x08\x04\x12\x10unexpected.scala\x50\x01"

# The --teq path's own damage, each a run of the real binary that the verdict must fail, by the group it runs over:
# over `given` (one document, typed), its documents deleted after teq wrote them, their MD5 zeroed (the protobuf still
# whole), the document written twice, a status no run of teq ends with, the status of a program that does not type
# over every document written; over `java-order-source` (one document, its pinned refusal), an unexpected document
# written beside the refusal. /bin/false over `given` too.
DAMAGE = [
    ("given", "the documents deleted", "for f in docs: f.unlink()"),
    ("given", "the MD5 zeroed", "for f in docs:\n    b, n = re.subn(rb'[0-9A-F]{32}', b'0' * 32, f.read_bytes())\n    assert n == 1, f\n    f.write_bytes(b)"),
    ("given", "the document written twice", "for f in docs: f.with_name('Twice-' + f.name).write_bytes(f.read_bytes())"),
    ("given", "a status of 101", "code = 101"),
    ("given", "a status of 2 over every document", "code = 2"),
    ("java-order-source", "an unexpected document beside a pinned refusal",
     f"if code == 2:\n    out = pathlib.Path(os.environ['TEQ_FIX_SEMANTICDB']) / 'META-INF' / 'semanticdb'\n"
     f"    out.mkdir(parents=True, exist_ok=True)\n    (out / 'unexpected.scala.semanticdb').write_bytes({UNEXPECTED!r})"),
]


def teq_path_checks(binary):
    """The verdict of the --teq path over its damage's groups: the binary as given passes each, and each of `DAMAGE`
    and /bin/false fails its group, through `compare_teq` itself."""
    def verdict(b, group):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = compare_teq(b, [group])
        return code, out.getvalue()
    groups = sorted({g for g, _, _ in DAMAGE})
    for group in groups:
        code, out = verdict(binary, group)
        if code != 0:
            print(f"the --teq path: {group} fails with the binary as given, so its damage proves nothing:\n{out}")
            return 1
    bad = 0
    with tempfile.TemporaryDirectory(prefix="teq-semanticdb-damage-") as tmp:
        runs = [("given", "/bin/false", Path("/bin/false"))]
        for group, what, body in DAMAGE:
            w = Path(tmp) / re.sub(r"\W+", "-", what)
            w.write_text(f"#!{sys.executable}\nimport os, pathlib, re, subprocess, sys\n"
                         f"code = subprocess.run([{str(binary)!r}, *sys.argv[1:]]).returncode\n"
                         f"docs = sorted(pathlib.Path(os.environ['TEQ_FIX_SEMANTICDB']).rglob('*.semanticdb'))\n"
                         f"{body}\nsys.exit(code)\n", encoding="utf-8")
            w.chmod(0o755)
            runs.append((group, what, w))
        for group, what, b in runs:
            code, out = verdict(b, group)
            if code != 1:
                bad += 1
                print(f"the --teq path: {what} over {group}: not failed:\n{out}")
    print(f"the --teq path: {' and '.join(groups)} pass with the binary, and {len(runs) - bad} of {len(runs)} damaged runs fail them")
    return 1 if bad else 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--regen", action="store_true")
    ap.add_argument("--only")
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--teq")
    a = ap.parse_args()
    if a.teq:
        groups = [a.only] if a.only else list(GROUPS)
        unknown = [g for g in groups if g not in GROUPS]
        if unknown:
            raise SystemExit(f"semanticdb: no group {unknown}")
        binary = Path(a.teq).resolve()
        checked = teq_path_checks(binary)
        return compare_teq(binary, groups) or checked
    if a.regen:
        groups = [a.only] if a.only else list(GROUPS)
        unknown = [g for g in groups if g not in GROUPS]
        if unknown:
            raise SystemExit(f"semanticdb: no group {unknown}")
        regen(groups)
        return 0
    if a.self_test:
        return self_test()
    return check()


if __name__ == "__main__":
    sys.exit(main())
