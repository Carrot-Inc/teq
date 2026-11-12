#!/bin/bash
# tests/tree-paths.sh [--self-test] [file...]: no tracked file says where or how the work on the tree is run. Two
# kinds of hit. An absolute path of a machine: a home directory with its login (Linux's `/home/<login>/`, macOS's
# and Windows's `Users/<login>` under the root or a drive) or a session's temporary directory (`/tmp/tmp.<random>`,
# macOS's `/var/folders/<xx>/`, `/tmp/claude-<n>`). The tree is public: such a path says where a recording was
# made, and a generated list that holds one changes with the machine that writes it.
# A recorded compiler output names its file as the tree does (tests/errors/x.scala); a runner's note names a
# checkout as the variable that holds it ($SCALA3) and a built file by its name alone.
# And a word of the workflow (the patterns of WORDS below, generic words only): the names of who assigns the work,
# of a decision on it, of a reviewer's role, of a review or its pass by its commit, of the work's numbered books
# and joints, of its parallel tracks and their scratch directory, of a work stream by its letter, of a gate machine
# by its number, of a review folded again; the directory of the reports; a clock time with its zone. A comment says
# what its test or its code is about: the program, the language's rule, scalac's answer, the place in dotty.
#
# Prints every hit as `<file>:<line>: <path or word>` and exits 1 when there is one; the last line is the count.
# With files as arguments, those are read in the tracked files' place (a plant, for a check of this script).
# --self-test plants one line of each kind in a scratch file, a placeholder and an allowed word, and fails unless
# each plant is reported and the two are passed in their own files alone.
#
# The files are the worktree's tracked ones (`git ls-files`), so the landing gate runs the check as a local line,
# on the machine that holds the worktree (tests/gate.sh, beside no-caps), and the workflow in its checkout.
#
# Left alone by the pattern: a home directory without a login (`/home/.cache`, a container's), `/root/`, `~/` and
# `$HOME/`. Left alone by name: this script, and the placeholders the tree writes on purpose, each allowed in its
# own file alone (ALLOWED below, with what it is), and a workflow word that is its file's own subject. Text files only (a file with a NUL byte is skipped, as
# `git grep -I` skips it).
cd "$(dirname "$0")/.."
exec python3 - "$@" <<'PY'
import re, subprocess, sys, tempfile, os

PATTERN = re.compile(
    r"/home/[A-Za-z_][A-Za-z0-9._-]*/"
    r"|/" r"Users/[A-Za-z_][A-Za-z0-9._-]*/"
    r"|[A-Za-z]:\\+Users\\+[A-Za-z0-9._-]+"
    r"|/tmp/tmp\.[A-Za-z0-9]{6,}"
    r"|/var/folders/[A-Za-z0-9_]{2}/"
    r"|/tmp/claude-[0-9]"
)
# The workflow's words, each written in two pieces where it is a plain word, so that this file is no hit of a grep
# for it. Case is ignored.
WORDS = re.compile(
    r"coordi" r"nator"
    r"|la" r"nes/scratch"
    r"|\bstream [a-z]'s\b"
    r"|\bgate-[0-9]+\b"
    r"|\bd-[0-9]{3}\b"
    # a decision by its act (never where `out` follows: plain English), a reviewer by its role
    r"|\bru" r"l(?:ings?|ed)\b(?! out)"
    r"|\badver" r"sar(?:y|ies)\b"
    # a review or its pass by its commit, a numbered book or joint of the work, one of its tracks, a second folding
    r"|\bpass(?:es|'s [a-z]+)? (?:on|over|of) (?:[a-z][a-z0-9-]*'s )?`?[0-9a-f]{7,}\b"
    r"|\b[0-9a-f]{7,}`?(?:'s)? pass\b"
    r"|\breviews? (?:of|on) [0-9a-f]{7,}\b"
    r"|\bled" r"gers?[ -][0-9]+"
    r"|\bse" r"am [0-9]+\b"
    r"|\bla" r"nes?\b"
    r"|\bre" r"fold(?:ed|s|ing)?\b"
    # where the reports are kept, and a clock time with its zone
    r"|~/re" r"ports/"
    r"|\b[0-9]{1,2}:[0-9]{2}(?::[0-9]{2})? ?(?:UTC|PST|PDT|PT|GMT|CET|CEST|EST|EDT)\b",
    re.IGNORECASE,
)
CLOCK = re.compile(r"[0-9]{1,2}:[0-9]{2}(?::[0-9]{2})? ?[A-Za-z]{2,4}")
# What a comment's next line begins with, dropped where two lines are read as one; and the same over a whole text.
CONTINUES = re.compile(r"^\s*(?://[/!]?|#|\*|--)?\s*")
JOINS = re.compile(r"[ \t]*\r?\n[ \t]*(?://[/!]?|#|\*|--)?[ \t]*")
SELF = "tests/tree-paths.sh"
# file -> the placeholders it may hold (each matched whole), and what they are.
ALLOWED = {
    # a unit test's two thousand made-up jars under a made-up home
    "src/argfile.rs": [r"/home/someone/"],
    # a recorded warning of a JVM, its user written `user`
    "tests/support/fallback-note/transcript.txt": [r"/home/user/"],
    # the inputs of the normaliser's own checks: temporary directories it must rewrite
    "tests/fork-one.sh": [r"/tmp/tmp\.(5f0Rb4gRSy|ABCDEF|UVWXYZ|XXXXXX)"],
    # made-up Windows homes in the sbt plugin's tests of paths with separators
    "integrations/sbt/src/test/scala/dev/teq/sbt/ArgsFileSuite.scala": [r"C:\\+Users\\+Zo"],
    "integrations/sbt/src/test/scala/dev/teq/sbt/ExportSuite.scala": [r"C:\\+Users\\+u"],
    # the fix run's own coordinating thread, which hands the documents to the workers: the code's subject
    "src/fix/engine.rs": [r"(?i)coordi" r"nator"],
    "src/fix/host.rs": [r"(?i)coordi" r"nator"],
    # the list of words a changelog entry must not hold: a check of this kind, the word its own subject
    "bench/changelog.sh": [r"(?i)coordi" r"nator"],
    # a checksum's four streams of eight bytes each, by the code's own word for them, and its variables
    "src/jarcache.rs": [r"(?i)la" r"nes?"],
    # a field of a test program's case class, its key in the JSON text and in the program's output
    "tests/cases/tuple_xxl.scala": [r"la" r"ne"],
    "tests/classpath/js/zio_json_xxl.scala": [r"la" r"ne"],
    "tests/classpath/js/zio_json_xxl.expected": [r"la" r"ne"],
    # the output of a test program's own class of that name, `println(s"Ledger $id")`
    "tests/cases/inherit_init_order.expected": [r"Led" r"ger [0-9]"],
    # a zone offset printed beside a zone's name, `+02:00` and then the zone
    "tests/classpath/js/java_time.expected": [r"02:00 G" r"MT"],
    "tests/classpath/jvm/java_time.expected": [r"02:00 G" r"MT"],
    # a JavaScript variable: whether the polls kept the driver's polling rule
    "tests/lsp/driver.mjs": [r"ru" r"led"],
    "tests/wine/lsp.mjs": [r"ru" r"led"],
}
# directory -> the keys its data files and the scripts that read them are organised by, each matched whole.
ALLOWED_UNDER = {
    # the cut classes of the closure lists, `<cut>(ledger-<n>)`, which closure.py compares and prints by that name
    "tests/tasty/representation/closure/": [r"(?i)led" r"ger[ -](?:69|77)"],
}


def hits(path, name):
    """The machine paths and the workflow words of one file, as (line number, what was found)."""
    try:
        data = open(path, "rb").read()
    except OSError:
        return []
    if b"\0" in data:
        return []
    allowed = [re.compile(a) for a in ALLOWED.get(name, [])]
    allowed += [re.compile(a) for d, keys in ALLOWED_UNDER.items() if name.startswith(d) for a in keys]
    out = []
    text = data.decode("utf-8", "replace")
    # Most files hold nothing: one search of the whole text, its lines joined as below, before the lines are read.
    if not PATTERN.search(text) and not WORDS.search(JOINS.sub(" ", text)):
        return out
    lines = text.splitlines()
    for n, line in enumerate(lines, 1):
        found = list(PATTERN.finditer(line)) + list(WORDS.finditer(line))
        # A phrase parted by a line's end: the line joined to the next one, the next one's comment prefix
        # dropped; only a match that crosses the join counts here (one inside a line is that line's), and no
        # clock time: a time that ends a line and a zone that begins the next are two things a program printed.
        if n < len(lines):
            head = line.rstrip()
            joined = head + " " + CONTINUES.sub("", lines[n])
            found += [m for m in WORDS.finditer(joined) if m.start() < len(head) < m.end() and not CLOCK.fullmatch(m.group(0))]
        for m in found:
            if not any(a.fullmatch(m.group(0)) for a in allowed):
                out.append((n, m.group(0)))
    return out


def scan(files):
    found = 0
    for f in files:
        if f == SELF:
            continue
        for n, p in hits(f, f):
            print(f"{f}:{n}: {p}")
            found += 1
    return found


def self_test():
    plants = ["/home/" + "agent/wt/x/out/a.js:1", "/" + "Users/me/src/A.scala", "C:\\" + "Users\\me\\a.jar",
              "/tmp/" + "tmp.Ab12Cd34/walk", "/var/" + "folders/zz/abc/T/x", "/tmp/" + "claude-1000/s",
              "// by the Coordi" + "nator's word of the 3rd", "Source: la" + "nes/scratch/x/a.txt", "stream b" + "'s reference",
              "recorded on gate" + "-5", "// d" + "-090: the expectation corrected",
              "// (the ru" + "ling of 27 Oct)", "// as ru" + "led", "// the closure adver" + "sary's probe", "// the pass on 97f0" + "487ac (round 24)",
              "// 4f95a" + "8370's pass, finding 3", "// (the review of 1888" + "ac4df)", "// (the push pass of 9afa" + "287f3, its probe)",
              "// and the 4c8b" + "a8586 pass's pairs", "// the pass's probe on 39ba" + "4cf2d", "// (the review on b047" + "da41b)", "// led" + "ger 54's pins", "// se" + "am 1, the alias kept", "// la" + "ne P's summary",
              "// on the M2e re" + "fold", "the table of ~/re" + "ports/board.md", "the end state (09:55 " + "PT)", "at 11:27 " + "UTC"]
    quiet = ["/home/.cache/coursier", "/root/.sbt", "$HOME/x", "~/notes/x.md", "tests/errors/a.scala:3:4",
             "the gate's lines", "a stream of events", "two coordinates", "gate-keeper", "id-1234", "d-12",
             "is ru" + "led out", "ru" + "ling out the rest", "a rule of the language", "a pass over the tree", "the pass on its way", "a review of the grammar",
             "the kernel's ledger", "seamless", "planes", "folded twice", "~/.cache/coursier", "at 14:37", "ratio 16:9 PTS"]
    # The two plain words are found inside a longer word too, as a grep for them finds them: a name built on
    # one is a hit until its file is allowed.
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "plant.txt")
        open(p, "w").write("\n".join(plants + quiet) + "\n")
        got = hits(p, "plant.txt")
        lines = sorted({n for n, _ in got})
        want = list(range(1, len(plants) + 1))
        if lines != want:
            print(f"tree-paths: the self-test's plants are on lines {want}, reported {lines}")
            return 1
        # a placeholder passes in its own file and nowhere else
        q = os.path.join(d, "argfile.rs")
        open(q, "w").write('"/home/' + 'someone/.cache/x.jar"\n')
        if hits(q, "src/argfile.rs") or not hits(q, "src/other.rs"):
            print("tree-paths: the self-test's placeholder is not allowed in its file alone")
            return 1
        # a phrase parted by a line's end is found at its first line; a harmless pair of lines is not
        x = os.path.join(d, "parted.rs")
        open(x, "w").write("/// guarding only actual recursion (the pass over\n/// 317e" + "8fb73's 6): a missing class\n// a pass over\n// the tree\n")
        if [n for n, _ in hits(x, "parted.rs")] != [1]:
            print("tree-paths: the self-test's phrase parted by a line's end is not reported at its first line alone")
            return 1
        w = os.path.join(d, "engine.rs")
        open(w, "w").write("/// Tells the coordi" + "nator of a worker that panicked.\n")
        if hits(w, "src/fix/engine.rs") or not hits(w, "src/typer/mod.rs"):
            print("tree-paths: the self-test's allowed word is not allowed in its file alone")
            return 1
    print(f"tree-paths: self-test, {len(plants)} plants reported, {len(quiet)} lines left alone, a placeholder and a word in their files alone, a phrase over two lines")
    return 0


args = sys.argv[1:]
if args[:1] == ["--self-test"]:
    sys.exit(self_test())
if args:
    files = args
else:
    listed = subprocess.run(["git", "ls-files", "-z"], capture_output=True)
    if listed.returncode != 0:
        # The check's input is the list of tracked files: a copy of the tree without its repository has none.
        print("tree-paths: not a git checkout here: no list of tracked files (run it in a worktree, or name the files)")
        sys.exit(2)
    files = listed.stdout.decode().split("\0")
    files = [f for f in files if f and os.path.isfile(f) and not os.path.islink(f)]
found = scan(files)
print(f"tree-paths: {found} machine path(s) or workflow word(s) in {len(files)} files" if found else f"tree-paths: no machine path and no workflow word in {len(files)} files")
sys.exit(1 if found else 0)
PY
