#!/bin/bash
# tests/fix/capture.sh [--check] [--scalac] <Rule>/<Case>...: what scalafix itself, or scalac's symbols, make of a
# case of tests/fix (tests/fix/REFERENCES.md), from the repository's root. The case's sources (input.scala and the
# .scala files beside it) are compiled by scalac 3.8.4 with -Xsemanticdb against the case's `jars`
# (tests/support/jars.sh), then scalafix's own CLI runs over input.scala under the case's configuration (its header
# comment, as scalafix's testkit reads it) with the three semantic rules' jars as its tool class path. It writes into
# the case's directory
#   scalafix.txt   scalafix's lints of input.scala, a line each in the order of their places:
#                  <line>:<col>:<width>: <severity>: [<id>] <message>
#                  (removed where scalafix reports none)
#   output.scala   what scalafix leaves of input.scala, without its header comment, where it rewrites
# and prints what it wrote. With --check it writes nothing: it compares and exits 1 where a file differs or is
# missing. A case whose configuration names a `file:` rule is teq's own and is refused.
#
# With --scalac scalafix does not run. The case's one rule is OptionGetWarn or ForbiddenSymbol as `teq fix` has them
# (docs/FIX.md), which is scalafix's rule without the defects of its information, and its lints are computed from
# the occurrences scalac's SemanticDB records in input.scala (tests/semanticdb/semanticdb.py decodes them), into
#   scalac.txt     a line a lint, as scalafix.txt's
# OptionGetWarn: each `get` of a selection `x.get` whose symbol is `scala/Option#get().` or `scala/None.get().`, a
# warning with the rule's message (`OptionGetWarn.message`). ForbiddenSymbol (`ForbiddenSymbol.symbols`): each name
# whose symbol, normalized as scalafix's `SymbolMatcher.normalized` (`SymbolOps.normalize`, `inferTrailingDot`), is
# an entry, at any occurrence as under scalafix; and each reference to a member that is no entry and overrides one,
# by dotty's own `allOverriddenSymbols`, which tests/fix/support/Overridden.scala prints of the case's sources
# (compiled here against the same compiler; SemanticDB has no overridden member of a library's). Either is an error
# whose message names the entry, `<entry> is forbidden according to the ForbiddenSymbol rule.`, the first in
# `allOverriddenSymbols`' order where a member overrides several. A case with a `scalac.txt` is held to scalac's
# symbols alone: --check compares that file and runs no scalafix, and a capture of it without --scalac is refused
# (scalafix reports fewer lints there), as --scalac is over a case with a scalafix.txt. `--scalac --check` over a
# case with a scalafix.txt and no scalac.txt holds the scalafix.txt to the symbols: a case where scalafix's lints are
# the rule's whole answer. The oracle knows no suppression and no lint filter: a case with an anchor, an
# annotation's or a `lint.` setting is refused.
#
# The versions are the reference's (tests/fix/REFERENCES.md): scalafix-cli_2.13.18 0.14.7, xuwei-k scalafix-rules
# 0.6.23, scalafix-forbidden-symbol 1.0.0, typelevel-scalafix-http4s 0.5.0 (each rule's source the pinned revision's),
# scala3-compiler_3 3.8.4. They are resolved by coursier into a cache of this script's own (CAPTURE_CACHE, by default
# ~/.cache/teq-fix-capture), never into the cache the suites read, whose jars a resolution must not change; the case's
# own jars are read from that cache (COURSIER_CACHE) and nothing is fetched into it.
# Environment: COURSIER (the cs launcher, default `cs`), JAVA (default `java`), CAPTURE_KEEP, CAPTURE_CASES (the
# directory the cases are named under, by default tests/fix: a probe's). Every step is bounded.
set -u
cd "$(dirname "$0")/../.." || exit 2
check=false
scalac=false
while :; do
  case ${1:-} in
    --check) check=true ;;
    --scalac) scalac=true ;;
    *) break ;;
  esac
  shift
done
[ $# -ge 1 ] || { echo "usage: tests/fix/capture.sh [--check] [--scalac] <Rule>/<Case>..." >&2; exit 2; }
cases=${CAPTURE_CASES:-tests/fix}
cs=${COURSIER:-cs}
java=${JAVA:-java}
export COURSIER_CACHE=${COURSIER_CACHE:-$HOME/.cache/coursier/v1}
. tests/support/jars.sh
cache=${CAPTURE_CACHE:-$HOME/.cache/teq-fix-capture}
mkdir -p "$cache" || exit 2

# A class path coursier resolves into the script's own cache: `resolved <name> [--intransitive] <coordinates...>`.
resolved() {
  local name=$1 out
  shift
  out=$(COURSIER_CACHE=$cache/coursier timeout 300 "$cs" fetch -p "$@" 2> "$cache/$name.err") || {
    echo "capture: cs fetch $* failed:" >&2
    tail -5 "$cache/$name.err" >&2
    exit 2
  }
  echo "$out"
}
compiler=$(resolved compiler org.scala-lang:scala3-compiler_3:3.8.4) || exit 2
library=$(resolved library org.scala-lang:scala3-library_3:3.8.4) || exit 2
scalafix=$(resolved scalafix ch.epfl.scala:scalafix-cli_2.13.18:0.14.7) || exit 2
rules=$(resolved rules --intransitive com.github.xuwei-k:scalafix-rules_2.13:0.6.23 \
  io.github.tanin47:scalafix-forbidden-symbol_2.13:1.0.0 org.typelevel:typelevel-scalafix-http4s_2.13:0.5.0) || exit 2

status=0
root=$(mktemp -d "${TMPDIR:-/tmp}/teq-fix-capture.XXXXXX") || exit 2
# CAPTURE_KEEP=1 leaves the work directories (their scalac.log, scalafix.out, SemanticDB and, of the scalac oracle,
# overridden.tsv and the scalac.txt it computed) and prints their root.
if [ -n "${CAPTURE_KEEP:-}" ]; then echo "capture: work under $root"; else trap 'rm -rf -- "${root:?}"' EXIT; fi

# tests/fix/support/Overridden.scala compiled against the compiler, once a text of it, in the script's cache: `dump`.
dump=""
overridden_classes() {
  [ -z "$dump" ] || return 0
  local sum dir
  sum=$(sha256sum tests/fix/support/Overridden.scala | cut -c1-16)
  dir=$cache/overridden-${sum:?}
  if [ ! -f "$dir/teq/fix/capture/Overridden.class" ]; then
    mkdir -p "$root/overridden"
    if ! timeout 300 "$java" -cp "$compiler" dotty.tools.dotc.Main -d "$root/overridden" -classpath "$compiler" \
      tests/fix/support/Overridden.scala > "$root/overridden.log" 2>&1; then
      echo "capture: scalac 3.8.4 refuses tests/fix/support/Overridden.scala:" >&2
      grep -v '^WARNING' "$root/overridden.log" | head -20 >&2
      return 1
    fi
    # Another run may have put it there meanwhile.
    mv -T "$root/overridden" "$dir" 2> /dev/null || [ -f "$dir/teq/fix/capture/Overridden.class" ] || return 1
  fi
  dump=$dir
}

for name in "$@"; do
  case=$cases/${name%/}
  [ -f "$case/input.scala" ] || { echo "capture: no $case/input.scala" >&2; status=2; continue; }
  # The case's oracle: scalac's symbols where asked, and under --check where the case has a scalac.txt; else scalafix.
  symbols=$scalac
  if [ -f "$case/scalac.txt" ] && ! $scalac; then
    if $check; then
      symbols=true
    else
      echo "capture: $name has a scalac.txt, its lints scalac's symbols' and not scalafix's: --scalac" >&2
      status=2
      continue
    fi
  fi
  if $symbols && ! $check && [ -f "$case/scalafix.txt" ]; then
    echo "capture: $name has a scalafix.txt, its lints scalafix's: --scalac --check holds them to scalac's symbols" >&2
    status=2
    continue
  fi
  work=$root/$(printf %s "${name%/}" | tr '/' '_')
  mkdir -p "$work/src" "$work/classes"
  for f in "$case"/*.scala; do
    [ "${f##*/}" = output.scala ] || cp "$f" "$work/src/"
  done
  # The configuration: the leading comment, as tests/fix.scala's `header` takes it.
  python3 - "$case/input.scala" > "$work/.scalafix.conf" << 'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
start, end = text.find("/*"), text.find("*/")
print(text[start + 2:end].strip() if start == 0 and end > 0 else "")
PY
  if grep -q '"file:' "$work/.scalafix.conf"; then
    echo "capture: $name configures a file: rule, which scalafix does not have" >&2
    status=2
    continue
  fi
  cp=""
  if [ -f "$case/jars" ]; then
    for key in $(cat "$case/jars"); do
      jar=$(jar_of "$key")
      [ -e "$jar" ] || { echo "capture: $name: the jar of $key is not in the coursier cache ($jar)" >&2; status=2; continue 2; }
      cp="$cp:$jar"
    done
  fi
  if ! (cd "$work" && timeout 300 "$java" -cp "$compiler" dotty.tools.dotc.Main -Xsemanticdb -sourceroot "$work" \
    -d "$work/classes" -classpath "$library$cp" src/*.scala) > "$work/scalac.log" 2>&1; then
    echo "capture: $name: scalac 3.8.4 refuses the case:" >&2
    grep -v '^WARNING' "$work/scalac.log" | head -20 >&2
    status=2
    continue
  fi
  if $symbols; then
    if grep -Eq '^[[:space:]]*rules?[[:space:]]*=.*ForbiddenSymbol' "$work/.scalafix.conf"; then
      overridden_classes || { status=2; continue; }
      if ! (cd "$work" && timeout 300 "$java" -cp "$compiler:$dump" teq.fix.capture.Overridden "$work/overridden.tsv" \
        -classpath "$library$cp" src/*.scala) > "$work/overridden.log" 2>&1; then
        echo "capture: $name: the dump of the overridden members failed:" >&2
        grep -v '^WARNING' "$work/overridden.log" | head -20 >&2
        status=2
        continue
      fi
    fi
    python3 - "$work" "$case" "$check" "$name" << 'PY'
import json, pathlib, re, sys
sys.dont_write_bytecode = True
sys.path.insert(0, "tests/semanticdb")
import semanticdb
work, case, check, name = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3] == "true", sys.argv[4]

def refuse(why):
    print("capture: %s: %s" % (name, why), file=sys.stderr)
    sys.exit(2)

# The configuration the oracle reads: the one rule, its entries, its message. Any other setting it does not model.
rest = (work / ".scalafix.conf").read_text(encoding="utf-8")
def take(pattern):
    global rest
    m = re.search(pattern, rest, re.S | re.M)
    if m:
        rest = rest[:m.start()] + rest[m.end():]
    return m
rule = take(r'^[ \t]*rules?[ \t]*=[ \t]*(?:\[[ \t]*"?(\w+)"?[ \t]*\]|"?(\w+)"?)[ \t]*$')
symbols = take(r'^[ \t]*ForbiddenSymbol\.symbols[ \t]*=[ \t]*\[(.*?)\][ \t]*$')
message = take(r'^[ \t]*OptionGetWarn\.message[ \t]*=[ \t]*("(?:[^"\\\n]|\\.)*")[ \t]*$')
rule = (rule[1] or rule[2]) if rule else None
if rule not in ("OptionGetWarn", "ForbiddenSymbol"):
    refuse("the scalac oracle has OptionGetWarn and ForbiddenSymbol, one a case, not `%s`" % rule)
if rest.strip():
    refuse("the scalac oracle reads the rule, ForbiddenSymbol.symbols and OptionGetWarn.message, not `%s`" % rest.strip().split("\n")[0])
source = (work / "src" / "input.scala").read_text(encoding="utf-8")
if re.search(r"scalafix:(ok|off|on)|SuppressWarnings", source):
    refuse("the scalac oracle knows no suppression, and input.scala has an anchor or an annotation's")
lines = source.split("\n")
documents = [d for f in sorted((work / "classes").rglob("*.semanticdb")) for d in semanticdb.decode_documents(f.read_bytes()) if d["uri"] == "src/input.scala"]
if len(documents) != 1:
    refuse("scalac wrote %d SemanticDB documents of input.scala" % len(documents))

# scalameta 4.17.4's symbols (semanticdb's Scala.scala: `isGlobal`, `DescriptorParser`, `Names.encode`) and scalafix's
# `SymbolOps.normalize` and `inferTrailingDot` (41cf7983): a symbol's owner chain, each name followed by a dot.
# A Java identifier's tests are of UTF-16 units: a character outside the basic plane is two surrogates, no part of one.
def part(c):
    return ord(c) <= 0xFFFF and (("a" + c).isidentifier() or c == "$")
def is_global(symbol):
    return symbol != "" and not symbol.startswith(";") and symbol[-1] in ".#/)]"
def descriptor(s):
    """The last descriptor's name and the symbol of its owner."""
    def value(i):
        if s[i] == "`":
            j = s.rindex("`", 0, i)
            return s[j + 1:i], j - 1
        j = i
        while j >= 0 and part(s[j]):
            j -= 1
        if j == i:
            raise ValueError("invalid symbol format: " + s)
        return s[j + 1:i + 1], j
    i = len(s) - 1
    if s[i] == ".":
        i -= 1
        if s[i] == ")":
            i = s.rindex("(", 0, i) - 1
        name, i = value(i)
    elif s[i] in "#/":
        name, i = value(i - 1)
    elif s[i] in ")]":
        name, i = value(i - 1)
        i -= 1
    else:
        raise ValueError("invalid symbol format: " + s)
    return name, s[:i + 1]
def encode(name):
    if name == "":
        return "``"
    start = ord(name[0]) <= 0xFFFF and (name[0].isidentifier() or name[0] == "$")
    return name if start and all(part(c) for c in name[1:]) else "`" + name + "`"
def normalize(symbol):
    if not is_global(symbol):
        return symbol
    names = []
    while symbol not in ("", "_root_/"):
        value, symbol = descriptor(symbol)
        names.append(encode(value))
    return "".join(n + "." for n in reversed(names))
def infer_trailing_dot(entry):
    return entry + "." if entry and (part(entry[-1]) or entry[-1] == "`") else entry
# dotty's fullName as a normalized symbol: the `$` of an object's class dropped (`scala.Predef$.println`).
def full(name):
    return "".join(encode(seg[:-1] if len(seg) > 1 and seg.endswith("$") else seg) + "." for seg in name.split("."))

# The occurrences by their ranges, each range's in the document's order: scalafix's `symbol` of a name is the first.
at = {}
for o in documents[0]["occurrences"]:
    r = o["range"]
    if r:
        at.setdefault((r["start_line"], r["start_character"], r["end_line"], r["end_character"]), []).append((o["symbol"], o["role"]))
def written(key):
    return lines[key[0]][key[1]:key[3]] if key[0] == key[2] else ""
def lint(key, severity, text):
    if key[0] != key[2]:
        refuse("an occurrence over several lines at line %d" % (key[0] + 1))
    return (key[0] + 1, key[1] + 1, key[3] - key[1], severity, rule, text)

lints, overriders, unjoined = [], 0, []
if rule == "OptionGetWarn":
    text = json.loads(message[1]) if message else "Don't use Option.get"
    for key in sorted(at):
        if not any(symbol in ("scala/Option#get().", "scala/None.get().") for symbol, _ in at[key]):
            continue
        # The rule reads a `Term.Select(_, Term.Name("get"))`; of the occurrences, the name after a dot is one.
        before = "\n".join(lines[:key[0]] + [lines[key[0]][:key[1]]]).rstrip()
        if written(key) not in ("get", "`get`") or not before.endswith("."):
            refuse("Option's `get` at %d:%d is not written `x.get`: the oracle does not tell whether it is a selection" % (key[0] + 1, key[1] + 1))
        lints.append(lint(key, "warning", text))
else:
    entries = [normalize(infer_trailing_dot(e)) for e in re.findall(r'"([^"]*)"', symbols[1])] if symbols else []
    members = {descriptor(e)[0] for e in entries if is_global(e)}
    rows = {}
    for row in (work / "overridden.tsv").read_text(encoding="utf-8").split("\n"):
        if row:
            file, span, symbol, overridden = row.split("\t")
            if file == "input.scala":
                (sl, sc), (el, ec) = [[int(n) - 1 for n in end.split(":")] for end in span.split("-")]
                rows.setdefault((sl, sc, el, ec), []).append((full(symbol), [full(o) for o in overridden.split(", ") if o]))
    for key in sorted(at):
        symbol, role = at[key][0]
        normalized = normalize(symbol)
        entry = normalized if normalized in entries else None
        # A reference to a member that is no entry: one of those it overrides may be. A definition is not reported.
        if entry is None and role == "REFERENCE" and (not is_global(symbol) or symbol[-1] == "."):
            if is_global(symbol):
                own = [r for r in rows.get(key, []) if normalized in (r[0], "_empty_." + r[0])]
                named = descriptor(symbol)[0]
            else:
                own = rows.get(key, [])
                named = written(key).strip("`")
            if len({tuple(r[1]) for r in own}) > 1:
                refuse("dotty has several members at %d:%d (%s)" % (key[0] + 1, key[1] + 1, symbol))
            if own:
                # dotty's fullName leaves out the empty package, which a symbol names.
                entry = next((e for o in own[0][1] for e in (o, "_empty_." + o) if e in entries), None)
                overriders += entry is not None
            elif named in members:
                refuse("no reference of dotty's at %d:%d for %s, which may override an entry" % (key[0] + 1, key[1] + 1, symbol))
            elif is_global(symbol):
                unjoined.append("%d:%d %s" % (key[0] + 1, key[1] + 1, symbol))
        if entry is not None:
            lints.append(lint(key, "error", entry[:-1] + " is forbidden according to the ForbiddenSymbol rule."))
    (work / "unjoined.txt").write_text("".join(u + "\n" for u in unjoined), encoding="utf-8")

lints.sort()
text = "".join("%d:%d:%d: %s: [%s] %s\n" % l for l in lints)
(work / "scalac.txt").write_text(text, encoding="utf-8")
# Under --check a case without a scalac.txt is held by its scalafix.txt, where scalafix's lints are the rule's.
path = case / "scalac.txt"
if check and not path.exists() and (case / "scalafix.txt").exists():
    path = case / "scalafix.txt"
have = path.read_text(encoding="utf-8") if path.exists() else None
differs = False
if have != (text or None):
    if check:
        differs = True
        print("capture: %s/%s %s" % (name, path.name, "differs from scalac's symbols'" if text and have is not None else "is missing" if have is None else "stands where scalac's symbols give none"))
    elif not text:
        path.unlink()
        print("capture: %s/scalac.txt removed (no lint)" % name)
    else:
        path.write_text(text, encoding="utf-8")
        print("capture: %s/scalac.txt written" % name)
print("capture: %s: %d lints by scalac's symbols%s%s" % (name, len(lints), ", %d of them overriders'" % overriders if rule == "ForbiddenSymbol" else "",
    ", %d references without dotty's (none an entry's name)" % len(unjoined) if unjoined else ""))
sys.exit(1 if differs else 0)
PY
    case $? in
      0) ;;
      1) [ $status -eq 2 ] || status=1 ;;
      *) status=2 ;;
    esac
    continue
  fi
  (cd "$work" && timeout 300 "$java" -cp "$scalafix" scalafix.cli.Cli --config "$work/.scalafix.conf" \
    --tool-classpath "$rules" --classpath "$work/classes:$library$cp" --sourceroot "$work" --scala-version 3.8.4 \
    --files src/input.scala) > "$work/scalafix.out" 2> "$work/scalafix.err"
  code=$?
  # 0, or 64 where a lint of error severity remains (scalafix's LinterError); anything else is no result.
  if [ $code -ne 0 ] && [ $code -ne 64 ]; then
    echo "capture: $name: scalafix exit $code:" >&2
    grep -v '^WARNING' "$work/scalafix.err" | head -20 >&2
    head -20 "$work/scalafix.out" >&2
    status=2
    continue
  fi
  python3 - "$work" "$case" "$check" "$name" << 'PY' || status=1
import pathlib, re, sys
work, case, check, name = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3] == "true", sys.argv[4]
# A lint as scalafix prints it: `<path>:<line>:<col>: <severity>: [<id>] <message>`, the source line, the carets.
lints, lines = [], (work / "scalafix.out").read_text(encoding="utf-8").split("\n")
head = re.compile(r"^(.*src/input\.scala):(\d+):(\d+): (error|warning|info): \[([^\]]+)\] (.*)$")
for i, line in enumerate(lines):
    m = head.match(line)
    if not m:
        continue
    carets = lines[i + 2] if i + 2 < len(lines) else ""
    width = len(carets.strip()) if set(carets.strip()) == {"^"} else 0
    lints.append((int(m[2]), int(m[3]), width, m[4], m[5], m[6]))
lints.sort()
want = {"scalafix.txt": "".join("%d:%d:%d: %s: [%s] %s\n" % l for l in lints) or None}
before, after = (case / "input.scala").read_text(encoding="utf-8"), (work / "src" / "input.scala").read_text(encoding="utf-8")
end = after.find("*/")
left = (after[end + 2:] if after.startswith("/*") and end > 0 else after).lstrip("\n")
# An output.scala that is the input as it was (a reference's own, of a fixture its rule leaves alone) may stand.
want["output.scala"] = left if after != before or (case / "output.scala").exists() else None
differs = False
for file, text in want.items():
    path = case / file
    have = path.read_text(encoding="utf-8") if path.exists() else None
    if file == "output.scala" and have is not None and text is not None and have.strip() == text.strip():
        continue  # the suite compares an output without its surrounding blank text
    if have == text:
        continue
    if check:
        differs = True
        print("capture: %s/%s %s" % (name, file, "differs from scalafix's" if text is not None and have is not None else "is missing" if have is None else "stands where scalafix gives none"))
    elif text is None:
        if file == "scalafix.txt":
            path.unlink()
            print("capture: %s/%s removed (no lint)" % (name, file))
        else:
            differs = True
            print("capture: %s/output.scala stands where scalafix rewrites nothing; remove it" % name)
    else:
        path.write_text(text, encoding="utf-8")
        print("capture: %s/%s written" % (name, file))
print("capture: %s: %d lints, %s" % (name, len(lints), "rewritten" if after != before else "not rewritten"))
sys.exit(1 if differs else 0)
PY
done
exit $status
