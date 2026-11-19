#!/usr/bin/env python3
"""The rewordings of the first words' landing, told apart from any other change of the diagnostics.

The landing changed two families to scalac's words: E007's layout and E006's plain words (reporting/messages.scala's
TypeMismatch and MissingIdent at scalac 3.8.4). This script tells those rewordings apart from any other change of a
program's diagnostics. It FAILS CLOSED BY CONSTRUCTION, in two steps.

TOTAL PARSE. An output is read whole into a sequence of blocks, then at most one summary, its last line.
  - A block is a diagnostic's header (its place, `<file>:<line>:<col>` or a source alone, or none at all, then its
    severity and its message's first line), its message's further lines, and the source excerpt and its caret as its
    last two lines where it has them.
  - A summary is `N error(s) found`, or `N warning(s) found, errors under --werror`.
  - After the summary, one line may stand: the analysis's answer (`--analysis-version`), a JSON object holding
    `analysisVersion`. Two answers are a rewording where they hold the same fields, each diagnostic's the same but its
    `message`, which is the old one rendered back, and `ms` the same timings by name (their values are the run's).
  - The output STOPS (is unclassified) on:
    - a line before the first block or after the summary;
    - a caret line anywhere but a block's last line, or with no line above it in the block;
    - no block in an output that is not empty.
  - The blocks and the summary give back the output byte for byte. That is checked, so nothing is ever dropped.

ONE DIRECTION. An old output (the base's words) and a new one (the head's) are a rewording iff the old output EQUALS
the new one with each new block rendered back into the old words, compared block by block in order, the summary and
the last line's end included, as exact text.
  - A block the head left as it was is `same`.
  - Any other block is rendered back by the named pattern its message's layout selects (scalac's layout is
    unambiguous: `Found:` and `Required:` on lines of their own). No old text is ever split to read it:
    - `E007`: `Found:    F` / `Required: R`, then at most one note on lines of its own (the TooUnspecific note, or
      the ambiguity note: conversions, `convert from`, or extension methods, `provide an extension method`), then a
      match type's trace. Back: `type mismatch: found F, required R`, the ambiguity note after `. `, the trace, the
      TooUnspecific note after the trace's last line after `. `, as master printed them.
    - `E006`: `Not found: X` back to `not found: X`, `Not found: type X` back to `type X not found`, X the rest of
      the line whole.
    - `a failed candidate's message whole`: a no-given message's `  candidate D failed:` with the candidate's message
      under it, four spaces in, back to `  candidate D failed: <its message rendered back>`. Only where that
      rendering is ONE line, so that the old line held it whole.
    - `E007 (no ambiguity note on Nothing)`: AmbiguousImplicits.notes gives no note on an argument of type Nothing.
      The new block `Found:    Nothing` / `Required: R` has nothing more. The old text must be its rendering, then
      `. Note that implicit extension methods cannot be applied because they are ambiguous; both D provide P`:
      - P is `an extension method `m` on Nothing` where R is `?{ m: ? }`, or `a conversion from Nothing to R`;
      - D is one line, two descriptions joined by ` and `, holding no ` provide `.
      D is the one text the new block does not hold.
  - The old words are many to one (`found "a, required Boolean", required Int` reads two ways), so a rendering is
    accepted only where its fields are the ONE reading of the old text whose parts qualify. A part qualifies when it
    is:
    - one line;
    - its string and character literals, backticks and brackets closed;
    - holding no `, required ` and no `. Note that ` outside a literal or backticks.
    Every split of the old text at `, required ` is counted; none or several that qualify is unclassified. A match
    type's trace holds no TooUnspecific note.
  - A block no pattern renders back to the old text exactly is unclassified, and so is a new block whose layout
    selects a pattern its text does not fill (a `Found:` line without its `Required:`, a note the landing does not
    word, a field that does not qualify).
  - Two outputs equal byte for byte are `same` unparsed (an identity run: the base already prints the new words). Any
    other pair is parsed whole on both sides.
  - The file's identity is the header's place whole. `--roots <old root> <new root>` removes only those two roots
    from the start of a place.

  rewording.py pairs [--roots <old root> <new root>] <old output> <new output>
      Two outputs (a program's terminal output, as tests/run_errors.sh records it after its exit line): prints the
      kind of each block or why the pair is refused. Exits 1 on a refusal.
  rewording.py records [--table <tsv>] <old record> <new record>
      A project's record (bench/<project>/errors.txt, deviations.json, run.json) against its migration.
      - The records' scopes (`key` lines, `projects`, `assertions`) must be equal.
      - Their items are equal as multisets: each old item is a new item, byte for byte or rendered back, every block
        of it through the same parse, and none is left over on either side.
      - An item that parses into no block is compared byte for byte: a run.json `<check>: not reached`.
      Prints the items' kinds (`same` for those byte for byte), every refused item, and the items left unpaired;
      `--table` writes every pair whole. Exits 1 on any.
  rewording.py test
      The fixtures: the review's mutation matrix (each refused through `pairs`, `records` in its three formats and the
      writing verb, the true pairs accepted), a second matrix over what it did not mutate, and each pattern with a
      fixture that just misses it.

tests/support/expects.py's writing verb asks `compare` of every program before it writes anything, and stops on a
refusal. tests/scala3/run.py and the typing harness read new words through `render_message`.
"""
import collections
import json
import os
import re
import sys

SEVERITY = r"(?P<sev>error|warning|migration warning|info)"
UNPLACED = re.compile(r"^" + SEVERITY + r": (?P<text>.*)$")
PLACED = re.compile(r"^(?P<place>.+?): " + SEVERITY + r": (?P<text>.*)$")
SUMMARY = re.compile(r"^(?:\d+ errors? found|\d+ warnings? found, errors under --werror)$")
# A caret line: spaces, carets, and what a record adds after it (a deviation's note of the build it stands for).
CARET = re.compile(r"^ *\^+(?: \(.*\))?$")
CHAR = re.compile(r"'(?:\\.|[^'\\])'")

TRACE = "\n\nNote: a match type could not be fully reduced:\n\n"
TOO_NEW = "Note that implicit conversions were not tried because the result of an implicit conversion\nmust be more specific than "
TOO_OLD = ". Note that implicit conversions were not tried because the result of an implicit conversion must be more specific than "
AMBIG_NEW = "Note that implicit {} cannot be applied because they are ambiguous;"
AMBIG_OLD = ". Note that implicit extension methods cannot be applied because they are ambiguous; both "
INLINED = re.compile(r"^  inlined from \S.*$")
CANDIDATE = re.compile(r"^  candidate (?P<d>.+?) failed:(?: (?P<m>.*))?$")
# The old words' delimiters: no part of a reading holds one outside a literal or backticks.
DELIMITERS = (", required ", ". Note that ")


class Unparsed(Exception):
    """An output, or a record, that does not read whole."""


class Block:
    """A diagnostic whole: its header's prefix (place and severity), its message's lines, its excerpt and caret."""

    def __init__(self, prefix, place, sev, first):
        self.prefix, self.place, self.sev, self.msg, self.excerpt = prefix, place, sev, [first], []

    def text(self, msg=None):
        lines = (msg if msg is not None else "\n".join(self.msg)).split("\n")
        return "\n".join([self.prefix + lines[0]] + lines[1:] + self.excerpt)

    def message(self):
        return "\n".join(self.msg)


def header(line):
    """A line that opens a block: (prefix up to the message, place or None, severity, the message's first line)."""
    m = UNPLACED.match(line) or PLACED.match(line)
    if not m:
        return None
    return line[:m.start("text")], m.groupdict().get("place"), m.group("sev"), m.group("text")


def parse(text):
    """An output read whole: (blocks, summary or None, whether its last line ends). Raises Unparsed."""
    if text in ("", "\n"):
        return [], (None, None), text == "\n"
    end = text.endswith("\n")
    lines = (text[:-1] if end else text).split("\n")
    blocks, summary, answer = [], None, None
    for i, l in enumerate(lines, 1):
        if summary is not None:
            if answer is None and analysis(l) is not None:
                answer = l
                continue
            raise Unparsed(f"line {i} stands after the summary: {l[:100]!r}")
        h = header(l)
        if h:
            blocks.append(Block(*h))
        elif SUMMARY.match(l):
            summary = l
        elif not blocks:
            raise Unparsed(f"line {i} stands before any diagnostic: {l[:100]!r}")
        else:
            blocks[-1].msg.append(l)
    if not blocks:
        raise Unparsed("no diagnostic in an output that is not empty")
    for b in blocks:
        carets = [j for j, l in enumerate(b.msg) if j > 0 and CARET.match(l)]
        if not carets:
            continue
        if carets != [len(b.msg) - 1] or len(b.msg) < 3:
            raise Unparsed(f"a caret line inside a message, or without its source line: {b.prefix}{b.msg[0][:80]!r}")
        b.msg, b.excerpt = b.msg[:-2], b.msg[-2:]
    back = "\n".join([b.text() for b in blocks] + [x for x in (summary, answer) if x is not None]) + ("\n" if end else "")
    if back != text:
        raise Unparsed("the blocks do not give the output back")
    return blocks, (summary, answer), end


def message_at(lines, i):
    """The message whole of the diagnostic whose header is lines[i], by the parse's rule: its lines up to the next
    header or the summary, the source excerpt and its caret apart. For a reader, which reads one diagnostic."""
    rest, j = [], i + 1
    while j < len(lines) and not header(lines[j]) and not SUMMARY.match(lines[j]):
        rest.append(lines[j])
        j += 1
    if len(rest) >= 2 and CARET.match(rest[-1]):
        rest = rest[:-2]
    return "\n".join([header(lines[i])[3]] + rest)


def render_stream(text):
    """A head's output rendered back whole to the output master's binary printed, for master's readers, which then
    read it with their own code: each diagnostic's message by render_message (strict off: one its pattern does not
    fill is given as it is), its header, excerpt and caret and every other line as they stand. The classifier proves
    that rendering exact on every expectation's output; what it cannot give back is the Nothing arm's dropped note
    and a term named `type X` (rows)."""
    lines, out, i = text.split("\n"), [], 0
    while i < len(lines):
        h = header(lines[i])
        if not h:
            out.append(lines[i])
            i += 1
            continue
        rest, j = [], i + 1
        while j < len(lines) and not header(lines[j]) and not SUMMARY.match(lines[j]):
            rest.append(lines[j])
            j += 1
        tail = []
        if len(rest) >= 2 and CARET.match(rest[-1]):
            rest, tail = rest[:-2], rest[-2:]
        # an output's last line end gives an empty last element: no part of the message
        trailing = []
        while rest and rest[-1] == "" and j == len(lines):
            trailing.insert(0, rest.pop())
        msg = render_message("\n".join([h[3]] + rest), strict=False)[1].split("\n")
        out += [h[0] + msg[0]] + msg[1:] + tail + trailing
        i = j
    return "\n".join(out)


def analysis(line):
    """The analysis's answer a line holds, or None."""
    try:
        d = json.loads(line)
    except ValueError:
        return None
    return d if isinstance(d, dict) and "analysisVersion" in d and isinstance(d.get("diagnostics", []), list) else None


def answer_verdict(old, new):
    """Whether the new analysis answer is the old one reworded: None, or why not."""
    if old == new:
        return None
    if old is None or new is None:
        return "an analysis answer on one side alone"
    o, n = analysis(old), analysis(new)
    if set(o) != set(n):
        return "the analysis answers hold other fields"
    for k in o:
        if k == "ms":
            if not (isinstance(o[k], dict) and isinstance(n[k], dict) and set(o[k]) == set(n[k])):
                return "the analysis answers time other steps"
        elif k == "diagnostics":
            if len(o[k]) != len(n[k]):
                return "the analysis answers hold other diagnostics"
            for od, nd in zip(o[k], n[k]):
                if not (isinstance(od, dict) and isinstance(nd, dict) and set(od) == set(nd)):
                    return "an analysis diagnostic of other fields"
                for f in od:
                    if f == "message" and od[f] != nd[f]:
                        kind, text = render_message(nd[f]) if isinstance(nd[f], str) else (None, "")
                        if kind is None or text != od[f]:
                            return f"an analysis diagnostic's message rendered back is {text[:100]!r} where the base's is {str(od[f])[:100]!r}"
                    elif od[f] != nd[f]:
                        return f"an analysis diagnostic's {f} differs"
        elif o[k] != n[k]:
            return f"the analysis answers' {k} differs"
    return None


def qualifies(s):
    """Whether a part of a reading qualifies: one line, its literals, backticks and brackets closed, no delimiter of the
    old words outside a literal or backticks."""
    if not s or "\n" in s or "\r" in s:
        return False
    stack, i, pairs = [], 0, {")": "(", "]": "[", "}": "{"}
    while i < len(s):
        c = s[i]
        if s.startswith(DELIMITERS, i):
            return False
        if s.startswith('"""', i):
            j = s.find('"""', i + 3)
            if j < 0:
                return False
            i = j + 3
            continue
        if c == '"':
            j = i + 1
            while j < len(s) and s[j] != '"':
                j += 2 if s[j] == "\\" else 1
            if j >= len(s):
                return False
            i = j + 1
            continue
        if c == "`":
            j = s.find("`", i + 1)
            if j < 0:
                return False
            i = j + 1
            continue
        if c == "'":
            m = CHAR.match(s, i)
            if m:
                i = m.end()
                continue
        if c in "([{":
            stack.append(c)
        elif c in ")]}":
            if not stack or stack.pop() != pairs[c]:
                return False
        i += 1
    return not stack


def readings(types):
    """The readings of the old words `<found>, required <required>` whose two parts qualify."""
    out, at = [], types.find(", required ")
    while at >= 0:
        found, required = types[:at], types[at + len(", required "):]
        if qualifies(found) and qualifies(required):
            out.append((found, required))
        at = types.find(", required ", at + 1)
    return out


def render_e007(msg, strict):
    """scalac's E007 as master printed it: (kind, old text) or (None, why)."""
    lines = msg.split("\n")
    if len(lines) < 2 or not lines[0].startswith("Found:    ") or not lines[1].startswith("Required: "):
        return None, "a Found: line without its Required: line"
    found, required = lines[0][len("Found:    "):], lines[1][len("Required: "):]
    after = msg[len(lines[0]) + 1 + len(lines[1]):]
    notes, trace = (after[:after.index(TRACE)], after[after.index(TRACE):]) if TRACE in after else (after, "")
    if strict:
        got = readings(f"{found}, required {required}")
        if got != [(found, required)]:
            return None, f"the types are not the one reading of the old words ({len(got)} readings qualify): {found!r}, {required!r}"
        if "Note that implicit conversions were not tried" in trace:
            return None, "a match type's trace holding the TooUnspecific note"
    base = f"type mismatch: found {found}, required {required}"
    if notes == "":
        return "E007", base + trace
    if not notes.startswith("\n"):
        return None, f"a note not on lines of its own: {notes[:80]!r}"
    note = notes[1:]
    if note.startswith(TOO_NEW):
        than = note[len(TOO_NEW):]
        if strict and not (qualifies(than) and than == required):
            return None, f"the TooUnspecific note names {than!r}, not the required type"
        return "E007", base + trace + TOO_OLD + than
    two = note.split("\n")
    for kind in ("conversions", "extension methods"):
        if len(two) == 2 and two[0] == AMBIG_NEW.format(kind) and two[1].startswith("both "):
            if trace:
                return None, "an ambiguity note beside a match type's trace (master gave the two apart)"
            rest = two[1][len("both "):]
            if kind == "conversions":
                if rest.count(" convert from ") != 1:
                    return None, f"an ambiguity of conversions without one `convert from`: {rest[:80]!r}"
                who, what = rest.split(" convert from ")
                old = AMBIG_OLD + who + " provide a conversion from " + what
            else:
                m = re.fullmatch(r"(?P<who>[^\n]+?) provide an extension method `[^`\n]+` on (?P<on>[^\n]+)", rest)
                if not m:
                    return None, f"an ambiguity of extension methods without what they provide: {rest[:80]!r}"
                who = m.group("who")
                old = AMBIG_OLD + rest
            if strict and (not who or " provide " in who or " convert from " in who):
                return None, f"the ambiguous candidates are not one reading: {who!r}"
            return "E007", base + old
    return None, f"a note the landing does not word: {note[:80]!r}"


def render_message(msg, strict=True):
    """A message rendered back into master's words by the pattern its layout selects: (kind, old text); `same` for a
    message no pattern selects; (None, why) for one its pattern does not fill. With strict off (a reader, not the
    classifier), the layout alone is read, and a message its pattern does not fill is given as it is."""
    kind, text = render_strict(msg, strict)
    if kind is None and not strict:
        return "same", msg
    return kind, text


def render_strict(msg, strict):
    # teq's inliner appends `  inlined from <place>` lines to a message (typer/inline.rs), after its notes and trace in
    # master's words as in the new: the message above them is rendered, they stay as they are
    lines = msg.split("\n")
    k = len(lines)
    while k > 1 and INLINED.match(lines[k - 1]):
        k -= 1
    if k < len(lines):
        kind, text = render_strict("\n".join(lines[:k]), strict)
        return (kind, text + "\n" + "\n".join(lines[k:])) if kind else (kind, text)
    if msg.startswith("Found:"):
        return render_e007(msg, strict)
    if msg.startswith("Not found: "):
        name = msg[len("Not found: "):]
        # an empty name stays empty (a syntax error's leftover: master printed `type  not found`, `not found: `)
        if "\n" in name or (strict and "\r" in name):
            return None, f"a missing name's message of more than one line: {msg[:80]!r}"
        # scalac's E006 words do not tell a term named `type X` (backticked) from a missing type X; the render-back
        # reads the type (a row: the fixture e006-term-named-type pins it)
        if name.startswith("type "):
            return "E006", f"type {name[len('type '):]} not found"
        return "E006", f"not found: {name}"
    lines = msg.split("\n")
    if any(CANDIDATE.match(l) for l in lines):
        return render_candidates(lines, strict)
    return "same", msg


def render_candidates(lines, strict):
    """A no-given message's failed candidates: each candidate's message rendered back onto its line."""
    out, i, moved, seen = [], 0, False, False
    while i < len(lines):
        m = CANDIDATE.match(lines[i])
        if not m:
            if seen:
                return None, f"a line after the failed candidates: {lines[i][:80]!r}"
            out.append(lines[i])
            i += 1
            continue
        seen = True
        if m.group("m") is not None:
            kind, text = render_message(m.group("m"), strict)
            if kind is None or "\n" in text:
                return None, f"a failed candidate's message on its line that does not render back: {m.group('m')[:80]!r}"
            moved |= kind != "same"
            out.append(f"  candidate {m.group('d')} failed: {text}")
            i += 1
            continue
        body, i = [], i + 1
        while i < len(lines) and lines[i].startswith("    "):
            body.append(lines[i][4:])
            i += 1
        kind, text = render_message("\n".join(body), strict)
        if len(body) < 2 or kind in (None, "same") or "\n" in text:
            return None, ("a failed candidate's message under it that does not render back to one line (the old line "
                          f"held one line alone): {body[:3]!r}")
        moved = True
        out.append(f"  candidate {m.group('d')} failed: {text}")
    return ("a failed candidate's message whole" if moved else "same"), "\n".join(out)


def nothing_arm(o, n):
    """Whether the old block is the new one with AmbiguousImplicits.notes' note on a Nothing argument, which the new
    one rightly drops."""
    m = re.fullmatch(r"Found:    Nothing\nRequired: (?P<r>[^\n]+)", n.message())
    if not m or o.prefix != n.prefix or o.excerpt != n.excerpt:
        return False
    required = m.group("r")
    if readings(f"Nothing, required {required}") != [("Nothing", required)]:
        return False
    base = f"type mismatch: found Nothing, required {required}" + AMBIG_OLD
    old = o.message()
    if not old.startswith(base):
        return False
    rest = old[len(base):]
    if rest.count(" provide ") != 1:
        return False
    who, what = rest.split(" provide ")
    two = re.fullmatch(r"\S[^\n]* and \S[^\n]*", who)
    ext = re.fullmatch(r"an extension method `(?P<m>[^`\n]+)` on Nothing", what)
    return bool(two) and (
        (ext is not None and required == "?{ " + ext.group("m") + ": ? }") or what == f"a conversion from Nothing to {required}")


def with_roots(blocks, root):
    """The blocks with one explicitly given root removed from the start of their places."""
    if root:
        for b in blocks:
            if b.place and b.place.startswith(root):
                b.prefix = "<root>" + b.prefix[len(root):]
    return blocks


def block_verdict(o, n):
    """(kind, None) for a block the old one is, rendered back; (None, why) otherwise."""
    if o.text() == n.text():
        return "same", None
    kind, text = render_message(n.message())
    if kind is None:
        return None, f"{n.prefix}unclassified: {text}"
    if n.text(text) == o.text():
        return kind, None
    if nothing_arm(o, n):
        return "E007 (no ambiguity note on Nothing)", None
    old, new = o.text().split("\n"), n.text(text).split("\n")
    at = next((j for j in range(max(len(old), len(new))) if j >= len(old) or j >= len(new) or old[j] != new[j]), 0)
    line = lambda ls: repr(ls[at][:120]) if at < len(ls) else "nothing"
    return None, f"{o.prefix}rendered back, line {at + 1} of the block is {line(new)} where the base's is {line(old)}"


def compare(old_text, new_text, roots=(None, None)):
    """Whether the new output is the old one reworded: (verdicts [(kind, why)] one a block, refusals [why])."""
    if old_text == new_text:
        return [("same", None)], []
    sides = []
    for side, text in (("base", old_text), ("head", new_text)):
        try:
            sides.append(parse(text))
        except Unparsed as e:
            return [], [f"the {side}'s output does not read whole: {e}"]
    (ob, (osum, oans), oend), (nb, (nsum, nans), nend) = sides
    with_roots(ob, roots[0])
    with_roots(nb, roots[1])
    refusals = []
    if len(ob) != len(nb):
        refusals.append(f"{len(ob)} diagnostics on the base, {len(nb)} on the head")
    if osum != nsum:
        refusals.append(f"the summary {osum!r} on the base, {nsum!r} on the head")
    if oend != nend:
        refusals.append("the last line's end differs")
    why = answer_verdict(oans, nans)
    if why:
        refusals.append(why)
    verdicts = [block_verdict(o, n) for o, n in zip(ob, nb)] if len(ob) == len(nb) else []
    refusals += [why for kind, why in verdicts if kind is None]
    return verdicts, refusals


def read_text(path):
    """A file's text, every byte kept (no newline translation; a byte not UTF-8 stays itself)."""
    with open(path, "rb") as f:
        return f.read().decode("utf-8", "surrogateescape")


def unescape(s):
    """records.scala's `unescape`: a backslash and the character after it."""
    out, i = [], 0
    while i < len(s):
        if s[i] == "\\" and i + 1 < len(s):
            out.append({"n": "\n", "r": "\r", "t": "\t"}.get(s[i + 1], s[i + 1]))
            i += 2
        else:
            out.append(s[i])
            i += 1
    return "".join(out)


def escape(s):
    """records.scala's `escape`."""
    return s.replace("\\", "\\\\").replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t")


def record_items(path):
    """A record's scope and its items, as records.scala reads them: (scope, [item]). Raises Unparsed."""
    text = read_text(path)
    if not path.endswith(".json"):
        scope, items, seen = [], [], set()
        for n, line in enumerate(text.split("\n"), 1):
            if not line or line.startswith("#"):
                continue
            word, _, rest = line.partition(" ")
            if word == "key":
                scope.append(unescape(rest))
            elif word == "error":
                items.append(unescape(rest))
            elif word in ("kind", "reason", "revision", "binary", "pin", "inputs") and word not in seen:
                seen.add(word)
            else:
                raise Unparsed(f"{path}: line {n}: no line starts `{word}` in a record")
        return sorted(scope), items
    try:
        d = json.loads(text)
    except ValueError as e:
        raise Unparsed(f"{path}: not JSON: {e}")
    keys = [k for k in ("deviations", "violations") if isinstance(d, dict) and k in d]
    if len(keys) != 1:
        raise Unparsed(f"{path}: neither or both of deviations and violations")
    items = d[keys[0]]
    scope = d.get("projects" if keys[0] == "deviations" else "assertions", [])
    if not isinstance(items, list) or not all(isinstance(x, str) for x in items) or not isinstance(scope, list):
        raise Unparsed(f"{path}: its items are not texts")
    return sorted(scope), items


def item_kinds(o, n):
    """The kinds of the new item's blocks, where it is the old one reworded, else None."""
    verdicts, refusals = compare(o, n)
    return None if refusals else [k for k, _ in verdicts]


def records(old_path, new_path):
    """The old and the new record's items paired whole: ([(old, new, kinds)], [why])."""
    (old_scope, old_items), (new_scope, new_items) = record_items(old_path), record_items(new_path)
    refusals = [] if old_scope == new_scope else ["the records' scopes differ"]
    left = collections.Counter(old_items)
    paired, rest = [], []
    for n in new_items:
        if left[n] > 0:
            left[n] -= 1
            paired.append((n, n, ["same"]))
        else:
            rest.append(n)
    # Each new item rendered back whole, then sought among the old items left.
    still = []
    for n in rest:
        try:
            blocks, summary, end = parse(n)
        except Unparsed as e:
            refusals.append(f"a new item does not read whole ({e}): {n[:160]!r}")
            continue
        texts, why = [], None
        for b in blocks:
            kind, text = render_message(b.message())
            if kind is None:
                why = text
                break
            texts.append(b.text(text))
        if why:
            refusals.append(f"a new item unclassified ({why}): {n[:160]!r}")
            continue
        if summary[1] is not None:
            still.append(n)
            continue
        old = "\n".join(texts + ([summary[0]] if summary[0] is not None else [])) + ("\n" if end else "")
        kinds = item_kinds(old, n) if left[old] > 0 else None
        if kinds:
            left[old] -= 1
            paired.append((old, n, kinds))
        else:
            still.append(n)
    # What renders to no old item left: the Nothing arm, whose old note the new item cannot give.
    for n in still:
        fits = sorted(o for o in left if left[o] > 0 and item_kinds(o, n))
        if len(fits) == 1:
            left[fits[0]] -= 1
            paired.append((fits[0], n, item_kinds(fits[0], n)))
        elif len(fits) > 1:
            refusals.append(f"a new item reads as {len(fits)} old items: {n[:160]!r}")
        else:
            refusals.append(f"UNPAIRED new: {n[:200]!r}")
    refusals += [f"UNPAIRED old: {o[:200]!r}" for o in sorted(left) for _ in range(left[o])]
    return paired, refusals


def main(argv):
    if argv == ["test"]:
        return test()
    if argv[:1] == ["records"]:
        args, table = argv[1:], None
        if args[:1] == ["--table"] and len(args) >= 2:
            table, args = args[1], args[2:]
        if len(args) != 2:
            print(__doc__, file=sys.stderr)
            return 2
        try:
            paired, refusals = records(*args)
        except (Unparsed, OSError) as e:
            print(f"REFUSED {e}")
            return 1
        for why in refusals:
            print(why if why.startswith("UNPAIRED") else f"REFUSED {why}")
        if table:
            with open(table, "w", encoding="utf-8", errors="surrogateescape") as f:
                f.write("kind\told item\tnew item\n")
                for o, n, ks in paired:
                    kind = ", ".join(sorted(set(k for k in ks if k != "same"))) or "same"
                    f.write(f"{kind}\t{escape(o)}\t{escape(n)}\n")
        items = collections.Counter(", ".join(sorted(set(k for k in ks if k != "same"))) or "same" for _, _, ks in paired)
        unpaired = sum(w.startswith("UNPAIRED") for w in refusals)
        print(f"records: {len(paired)} items paired ({', '.join(f'{k} {v}' for k, v in sorted(items.items()))}), "
              f"{len(refusals) - unpaired} refused, {unpaired} unpaired")
        return 1 if refusals else 0
    if argv[:1] == ["pairs"]:
        args, roots = argv[1:], (None, None)
        if args[:1] == ["--roots"] and len(args) >= 3:
            roots, args = (args[1], args[2]), args[3:]
        if len(args) != 2:
            print(__doc__, file=sys.stderr)
            return 2
        try:
            old, new = read_text(args[0]), read_text(args[1])
        except OSError as e:
            print(f"REFUSED {e}")
            return 1
        verdicts, refusals = compare(old, new, roots)
        for kind, why in verdicts:
            print("same" if kind == "same" else f"reworded {kind}" if kind else f"CHANGED {why}")
        for why in refusals:
            if all(why != w for _, w in verdicts):
                print(f"REFUSED {why}")
        return 1 if refusals else 0
    print(__doc__, file=sys.stderr)
    return 2


# The fixtures. A neutral program of two errors, as master and the head print it (the review's two_errors.scala).
FILE = "work/two_errors/two_errors.scala"


def two_errors(new, file=FILE):
    if new:
        return (f'{file}:1:14: error: Found:    "bad"\nRequired: Int\n  val x: Int = "bad"\n               ^^^^^\n'
                f'{file}:2:17: error: Found:    42\nRequired: String\n  val y: String = 42\n                  ^^\n2 errors found\n')
    return (f'{file}:1:14: error: type mismatch: found "bad", required Int\n  val x: Int = "bad"\n               ^^^^^\n'
            f'{file}:2:17: error: type mismatch: found 42, required String\n  val y: String = 42\n                  ^^\n2 errors found\n')


def one(msg, place="1:1", excerpt="  val n = summon[Need]\n  ^"):
    return f"p.scala:{place}: error: {msg}\n{excerpt}\n1 error found\n"


def review_matrix():
    """The review's mutations (packs/first-words-1b/review/classifier_probe.py and delimiter_classifier.py), over the
    neutral pair: (name, old, new, accepted)."""
    base, head = two_errors(False), two_errors(True)
    cut = lambda s: s[:s.index("\n" + FILE, 1)] + "\n1 error found\n"
    second = lambda s: s[s.index("\n" + FILE, 1) + 1:s.rindex("2 errors found")]
    swapped = second(head) + head[:head.index("\n" + FILE, 1)] + "\n2 errors found\n"
    lit = lambda msg: f'{FILE}:1:14: error: {msg}\n  val x: Int = "a, required Boolean"\n               ^^^^^^^^^^^^^^^^^^^^^\n1 error found\n'
    lit_old, lit_new = lit('type mismatch: found "a, required Boolean", required Int'), lit('Found:    "a, required Boolean"\nRequired: Int')
    no_given = "no given instance of type Need was found for parameter x\n  candidate g failed:"
    return [
        ("control-reword", base, head, True),
        ("line-moved", base, head.replace(":2:17:", ":3:17:"), False),
        ("column-moved", base, head.replace(":2:17:", ":2:18:"), False),
        ("severity-changed", base, head.replace(":2:17: error:", ":2:17: warning:"), False),
        ("required-changed-second", base, head.replace("Required: String", "Required: Boolean"), False),
        ("error-removed", base, cut(head), False),
        ("error-added", cut(base), head, False),
        ("errors-swapped", base, swapped, False),
        ("file-moved", base, head.replace("/two_errors/two_errors.scala:", "/different/two_errors.scala:"), False),
        ("unplaced-added-before", base, "error: unrelated error added\n" + head, False),
        ("unplaced-added-after", base, head + "error: unrelated error added\n", False),
        ("summary-changed", base, head.replace("2 errors found", "3 errors found"), False),
        ("space-in-path", base.replace("/two_errors/", "/two errors/"),
         head.replace("/two_errors/", "/two errors/").replace("Required: String", "Required: Boolean"), False),
        ("space-in-path, the true pair", base.replace("/two_errors/", "/two errors/"), head.replace("/two_errors/", "/two errors/"), True),
        ("required-cut-short", base, head.replace("Required: String\n", ""), False),
        ("extra-note", base, head.replace("Required: String", "Required: String\nNote: fabricated"), False),
        ("excerpt-changed", base, head.replace('  val x: Int = "bad"', '  val x: Int = "bad!"'), False),
        ("second-family-changed", base, head.replace("Found:    42\nRequired: String", "value missing is not a member of Boolean"), False),
        ("candidate-fabricated-tail", one(no_given + " unchanged"),
         one(no_given + "\n    unchanged\n    completely unrelated changed diagnostic"), False),
        ("candidate-added-note", one(no_given + " type mismatch: found B, required AnyVal"),
         one(no_given + "\n    Found:    B\n    Required: AnyVal\n    " + TOO_NEW.replace("\n", "\n    ") + "AnyVal"), False),
        ("delimiter-actual", lit_old, lit_new, True),
        ("delimiter-mutated", lit_old, lit_new.replace('Found:    "a, required Boolean"\nRequired: Int', 'Found:    "a\nRequired: Boolean", required Int'), False),
    ]


def second_matrix():
    """What the review did not mutate, and each pattern with a fixture that just misses it: (name, old, new,
    accepted)."""
    base, head = two_errors(False), two_errors(True)
    h = dict(place="4:24", excerpt="  def h[X](x: X): M[X] = mk\n                         ^^")
    reduce = "  trying to reduce  M[X]\n  failed since selector X\n  does not match  case Int => String"
    trace_old, trace_new = one("type mismatch: found String, required M[X]" + TRACE + reduce, **h), one("Found:    String\nRequired: M[X]" + TRACE + reduce, **h)
    at_one = lambda first, second: (f"{FILE}:1:14: error: {first}\n  val x = f(g)\n               ^\n"
                                    f"{FILE}:1:14: error: {second}\n  val x = f(g)\n               ^\n2 errors found\n")
    warned = lambda s, w="match may not be exhaustive": f"{FILE}:1:5: warning: {w}\n  x match\n  ^\n" + s
    nothing = lambda what, req="?{ toSet: ? }", word="provide": (
        one("type mismatch: found Nothing, required " + req + AMBIG_OLD + f"method a in class A and method b in class B {word} " + what),
        one("Found:    Nothing\nRequired: " + req))
    no_given = "no given instance of type Need was found for parameter x\n  candidate g failed:"
    conv = (one("type mismatch: found X, required B" + AMBIG_OLD + "method first and method second provide a conversion from X to B"),
            one("Found:    X\nRequired: B\n" + AMBIG_NEW.format("conversions") + "\nboth method first and method second convert from X to B"))
    ext = (one("type mismatch: found X, required ?{ foo: ? }" + AMBIG_OLD + "method xa and method xb provide an extension method `foo` on X"),
           one("Found:    X\nRequired: ?{ foo: ? }\n" + AMBIG_NEW.format("extension methods") + "\nboth method xa and method xb provide an extension method `foo` on X"))
    too = (one("type mismatch: found B, required AnyVal" + TOO_OLD + "AnyVal"), one("Found:    B\nRequired: AnyVal\n" + TOO_NEW + "AnyVal"))
    glued = (one("type mismatch: found (x : M[X]), required AnyVal" + TRACE + "  trying to reduce  M[X]" + TOO_OLD + "AnyVal"),
             one("Found:    (x : M[X])\nRequired: AnyVal\n" + TOO_NEW + "AnyVal" + TRACE + "  trying to reduce  M[X]"))
    two_readings = one('type mismatch: found T[A, required B], required ", required "')

    def answer(new_msg='Found:    \\"s\\"\\nRequired: Int'):
        """A build's terminal output and its analysis answer (`--analysis-version 3`), as master and the head give it."""
        out = lambda msg, json_msg, ms: (one(msg, place="20:18", excerpt='  val wrong: Int = "s"\n                   ^^^')
                                         + '{"ok":false,"analysisVersion":3,"diagnostics":[{"file":"p.scala","line":20,"col":18,"severity":"error",'
                                         + f'"message":"{json_msg}","source":"val wrong: Int = \\"s\\""}}],"ms":{{"parse":{ms},"type":9.7}}}}\n')
        return (out('type mismatch: found "s", required Int', 'type mismatch: found \\"s\\", required Int', "1.129"),
                out('Found:    "s"\nRequired: Int', new_msg, "1.068"))
    return [
        ("excerpt-line-second", base, head.replace("  val y: String = 42", "  val y: String = 43"), False),
        ("caret-wider", base, head.replace("                  ^^\n", "                  ^^^\n"), False),
        ("caret-moved", base, head.replace("               ^^^^^\n", "                ^^^^^\n"), False),
        ("caret-dropped", base, head.replace("               ^^^^^\n", ""), False),
        ("tab-in-excerpt", base, head.replace("  val y: String = 42", "  val y:\tString = 42"), False),
        ("tab-in-message", base, head.replace("Found:    42", "Found:\t42"), False),
        ("tab-on-both-sides", base.replace("  val y: String = 42", "  val y:\tString = 42"), head.replace("  val y: String = 42", "  val y:\tString = 42"), True),
        ("crlf-head", base, head.replace("\n", "\r\n"), False),
        ("crlf-one-line", base, head.replace("Required: Int\n", "Required: Int\r\n"), False),
        ("five-lines", trace_old, trace_new, True),
        ("five-lines-trace-changed", trace_old, trace_new.replace("failed since selector X", "failed since selector Y"), False),
        ("five-lines-trace-dropped", trace_old, one("Found:    String\nRequired: M[X]", **h), False),
        ("five-lines-trace-blank-line", trace_old, trace_new.replace("\n\nNote: a match", "\n\n\nNote: a match"), False),
        ("two-at-one-place", at_one("type mismatch: found A, required B", "not found: g"), at_one("Found:    A\nRequired: B", "Not found: g"), True),
        ("two-at-one-place-swapped", at_one("type mismatch: found A, required B", "not found: g"), at_one("Not found: g", "Found:    A\nRequired: B"), False),
        ("warning-beside", warned(base), warned(head), True),
        ("warning-beside-changed", warned(base), warned(head, "match may not be exhaustive!"), False),
        ("warning-beside-dropped", warned(base), head, False),
        ("zero-blocks-head", base, "", False),
        ("zero-blocks-base", "", head, False),
        ("text-of-no-diagnostic-head", base, "something else entirely\n", False),
        ("identical-unparsed", 'not a diagnostic\n{"ok": false}\n', 'not a diagnostic\n{"ok": false}\n', True),
        ("line-after-summary", base, head + '{"ok":false}\n', False),
        ("last-line-end", base, head[:-1], False),
        ("e006-term", one("not found: notThere"), one("Not found: notThere"), True),
        ("e006-type", one("type Missing not found"), one("Not found: type Missing"), True),
        ("e006-multiword-type", one("type Two Words not found"), one("Not found: type Two Words"), True),
        ("e006-backticked-term", one("not found: two words"), one("Not found: two words"), True),
        # a row: a term named `type Widget` is the true pair, refused, since the render-back reads the type
        ("e006-term-named-type", one("not found: type Widget"), one("Not found: type Widget"), False),
        ("inlined-from-e007", one("type mismatch: found A, required B\n  inlined from <std>/mutable.scala:66"), one("Found:    A\nRequired: B\n  inlined from <std>/mutable.scala:66"), True),
        ("inlined-from-e007-trace-and-note", one("type mismatch: found (x : M[X]), required AnyVal" + TRACE + "  trying to reduce  M[X]" + TOO_OLD + "AnyVal\n  inlined from p.scala:3\n  inlined from p.scala:9"),
         one("Found:    (x : M[X])\nRequired: AnyVal\n" + TOO_NEW + "AnyVal" + TRACE + "  trying to reduce  M[X]\n  inlined from p.scala:3\n  inlined from p.scala:9"), True),
        ("inlined-from-e006", one("not found: x\n  inlined from p.scala:3"), one("Not found: x\n  inlined from p.scala:3"), True),
        ("inlined-from-changed", one("type mismatch: found A, required B\n  inlined from p.scala:3"), one("Found:    A\nRequired: B\n  inlined from p.scala:4"), False),
        ("inlined-from-added", one("type mismatch: found A, required B"), one("Found:    A\nRequired: B\n  inlined from p.scala:4"), False),
        ("e006-type-without-a-name", one("type  not found"), one("Not found: type "), True),
        ("e006-term-without-a-name", one("not found: "), one("Not found: "), True),
        ("e006-type-without-a-name-flipped", one("not found: type "), one("Not found: type "), False),
        ("e006-kind-flipped", one("not found: x"), one("Not found: type x"), False),
        ("e006-name-cut", one("type Two Words not found"), one("Not found: type Two"), False),
        ("too-unspecific", *too, True),
        ("too-unspecific-other-type", too[0], too[1].replace("than AnyVal", "than AnyRef"), False),
        ("too-unspecific-blank-line", too[0], too[1].replace("AnyVal\nNote", "AnyVal\n\nNote"), False),
        ("too-unspecific-added", one("type mismatch: found B, required AnyVal"), too[1], False),
        ("trace-and-too-unspecific", *glued, True),
        ("trace-and-too-unspecific-in-the-trace", glued[0], one("Found:    (x : M[X])\nRequired: AnyVal" + TRACE + "  trying to reduce  M[X]" + TOO_OLD + "AnyVal"), False),
        ("ambiguous-conversions", *conv, True),
        ("ambiguous-extension-methods", *ext, True),
        ("ambiguous-kind-flipped", ext[0], ext[1].replace("implicit extension methods", "implicit conversions"), False),
        ("ambiguous-conversions-as-extension-methods", conv[0], conv[1].replace("implicit conversions", "implicit extension methods"), False),
        ("nothing-arm", *nothing("an extension method `toSet` on Nothing"), True),
        ("nothing-arm-conversion", *nothing("a conversion from Nothing to B", "B"), True),
        ("nothing-arm-one-word-off", *nothing("an extension method `toSet` on Nothing", word="offer"), False),
        ("nothing-arm-other-member", *nothing("an extension method `toList` on Nothing"), False),
        ("nothing-arm-not-nothing", nothing("an extension method `toSet` on Nothing")[0].replace("found Nothing", "found Int"),
         nothing("an extension method `toSet` on Nothing")[1].replace("Found:    Nothing", "Found:    Int"), False),
        ("nothing-arm-note-broken-before-provide", nothing("an extension method `toSet` on Nothing")[0].replace(" provide an", "\n provide an"),
         nothing("an extension method `toSet` on Nothing")[1], False),
        ("nothing-arm-blank-line-after", nothing("an extension method `toSet` on Nothing")[0].replace("on Nothing\n", "on Nothing\n\n"),
         nothing("an extension method `toSet` on Nothing")[1], False),
        ("nothing-arm-new-blank-line-after", nothing("an extension method `toSet` on Nothing")[0],
         nothing("an extension method `toSet` on Nothing")[1].replace("toSet: ? }\n", "toSet: ? }\n\n"), False),
        ("nothing-arm-two-provides", *nothing("an extension method `toSet` on Nothing provide an extension method `toSet` on Nothing"), False),
        ("candidate-one-line", one(no_given + " not found: x"), one(no_given + " Not found: x"), True),
        ("candidate-whole", one(no_given + " type mismatch: found Int, required String"), one(no_given + "\n    Found:    Int\n    Required: String"), True),
        ("candidate-renders-to-two-lines", one(no_given + " type mismatch: found Int, required M[X]"),
         one(no_given + "\n    Found:    Int\n    Required: M[X]\n    \n    Note: a match type could not be fully reduced:\n    \n      trying to reduce  M[X]"), False),
        ("found-three-spaces", base, head.replace('Found:    "bad"', 'Found:   "bad"'), False),
        ("found-unclosed-bracket", one("type mismatch: found List[Int, required Int"), one("Found:    List[Int\nRequired: Int"), False),
        ("two-readings: the delimiter in brackets", two_readings, one('Found:    T[A, required B]\nRequired: ", required "'), False),
        ("two-readings: the brackets split", two_readings, one('Found:    T[A\nRequired: B], required ", required "'), False),
        ("one reading: the delimiter in quotes in brackets", one('type mismatch: found T[", required "], required Int'),
         one('Found:    T[", required "]\nRequired: Int'), True),
        ("analysis-answer", *answer(), True),
        ("analysis-answer-message-changed", answer()[0], answer(new_msg='Found:    \\"t\\"\\nRequired: Int')[1], False),
        ("analysis-answer-field-changed", answer()[0], answer()[1].replace('"col":18', '"col":19'), False),
        ("analysis-answer-step-timed-apart", answer()[0], answer()[1].replace('"type":', '"typer":'), False),
        ("analysis-answer-twice", answer()[0], answer()[1] + answer()[1].split("\n")[-2] + "\n", False),
        ("analysis-answer-dropped", answer()[0], answer()[1].rsplit("\n", 2)[0] + "\n", False),
        ("roots-same-relative", base.replace("work/", "/one/work/"), head.replace("work/", "/two/work/"), "roots"),
        ("roots-other-relative", base.replace("work/", "/one/work/"), head.replace("work/two_errors/", "/two/work/other/"), "roots-refused"),
    ]


def test():
    import contextlib
    import io
    import tempfile
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import expects
    bad, n = 0, 0

    def check(name, got, want):
        nonlocal bad, n
        n += 1
        if got != want:
            bad += 1
            print(f"FAIL rewording test {name}: {'accepted' if got else 'refused'}, must be {'accepted' if want else 'refused'}")

    def write(path, s):
        with open(path, "w", encoding="utf-8", errors="surrogateescape", newline="") as fh:
            fh.write(s)

    for name, old, new, want in review_matrix() + second_matrix():
        if want in ("roots", "roots-refused"):
            check(name + " (pairs, no roots given)", not compare(old, new)[1], False)
            check(name + " (pairs, the roots given)", not compare(old, new, ("/one/", "/two/"))[1], want == "roots")
            continue
        check(name + " (pairs)", not compare(old, new)[1], want)
        with tempfile.TemporaryDirectory() as d:
            p = lambda f: os.path.join(d, f)
            for side, s in (("old", old), ("new", new)):
                for f, key in ((".json", "deviations"), (".run.json", "violations")):
                    write(p(side + f), json.dumps({key: ["core compile " + s] if s else []}))
                write(p(side + ".errors.txt"), "error core compile " + escape(s) + "\n" if s else "")
            for f in (".json", ".run.json", ".errors.txt"):
                check(f"{name} (records, {f[1:]})", not records(p("old" + f), p("new" + f))[1], want)
            for sub in ("errors", "base", "head"):
                os.mkdir(p(sub))
            write(p("errors/p.scala"), 'val x: Int = "bad"\nval y: String = 42\n')
            write(p("errors/p.check"), old)
            write(p("base/p.out"), "exit 1\n" + old)
            write(p("head/p.out"), "exit 1\n" + new)
            write(p("pending.txt"), "")
            with contextlib.redirect_stdout(io.StringIO()):
                code = expects.write(p("errors"), p("base"), p("head"), p("pending.txt"), True)
            written = read_text(p("errors/p.check")) != old
            check(f"{name} (the writing verb)", code == 0, want)
            if not want and written:
                bad += 1
                print(f"FAIL rewording test {name}: the writing verb wrote the .check")
    # render_stream (the readers' render-back) gives every accepted pair's old output back byte for byte, but the
    # Nothing arm's, whose dropped note it cannot give, and the analysis answer's, a JSON line of the run's timings that
    # no stream reader reads (the app oracle renders its messages one by one).
    for name, old, new, want in review_matrix() + second_matrix():
        if want is True and not name.startswith(("nothing-arm", "analysis-answer")):
            check(f"{name} (render_stream gives the old output back)", render_stream(new) == old, True)
    # The writing verb with a program's record on one side alone: it stops, nothing written.
    for missing in ("base", "head"):
        with tempfile.TemporaryDirectory() as d:
            p = lambda f: os.path.join(d, f)
            for sub in ("errors", "base", "head"):
                os.mkdir(p(sub))
            write(p("errors/p.scala"), 'val x: Int = "bad"\n')
            write(p("errors/p.check"), two_errors(False))
            for side in ("base", "head"):
                if side != missing:
                    write(p(f"{side}/p.out"), "exit 1\n" + two_errors(side == "head"))
            write(p("pending.txt"), "")
            with contextlib.redirect_stdout(io.StringIO()):
                code = expects.write(p("errors"), p("base"), p("head"), p("pending.txt"), True)
            check(f"the writing verb with no {missing} record for a program", code == 0 or read_text(p("errors/p.check")) != two_errors(False), False)
    # A record's own reading: its escapes, its kinds of line, its JSON keys, its scope, items of no diagnostic.
    with tempfile.TemporaryDirectory() as d:
        def rec(name, text):
            write(os.path.join(d, name), text)
            return os.path.join(d, name)
        o = rec("o.errors.txt", "error lila/test a.scala:1:2: error: not found: x\\n  a\\\\b\\tc\\n  ^\n")
        check("records: a backslash and a tab escaped", not records(o, rec("n.errors.txt", "error lila/test a.scala:1:2: error: Not found: x\\n  a\\\\b\\tc\\n  ^\n"))[1], True)
        check("records: read as records.scala reads", record_items(o)[1] == ["lila/test a.scala:1:2: error: not found: x\n  a\\b\tc\n  ^"], True)
        check("records: a tab against a space", not records(o, rec("n2.errors.txt", "error lila/test a.scala:1:2: error: Not found: x\\n  a\\\\b c\\n  ^\n"))[1], False)
        for name, f, text in (("a line of no record word", "u.errors.txt", "debt x\n"), ("a provenance word twice", "u.errors.txt", "kind migrate\nkind migrate\n"),
                              ("both item keys", "u.json", '{"deviations": [], "violations": []}'), ("no item key", "u.json", '{"projects": []}')):
            try:
                record_items(rec(f, text))
                check(f"records: {name} refused", True, False)
            except Unparsed:
                check(f"records: {name} refused", False, False)
        check("records: another scope", not records(rec("s1.json", '{"projects": ["a"], "deviations": []}'), rec("s2.json", '{"projects": ["a", "b"], "deviations": []}'))[1], False)
        reached = rec("r1.run.json", '{"assertions": ["t"], "violations": ["the asset served: not reached"]}')
        check("records: an item of no diagnostic, as it was", not records(reached, reached)[1], True)
        check("records: an item of no diagnostic, changed", not records(reached, rec("r2.run.json", '{"assertions": ["t"], "violations": ["the asset served: not reached!"]}'))[1], False)
        whole, empty = rec("z1.json", json.dumps({"deviations": ["core compile " + two_errors(False)]})), rec("z2.json", json.dumps({"deviations": ["core compile"]}))
        check("records: an item with zero blocks on the head's side", not records(whole, empty)[1], False)
        check("records: an item with zero blocks on the base's side", not records(empty, whole)[1], False)
        with open(os.path.join(d, "b1.txt"), "wb") as fh:
            fh.write(b"p.scala:1:1: error: not found: \xff\n  x\n  ^\n1 error found\n")
        with open(os.path.join(d, "b2.txt"), "wb") as fh:
            fh.write(b"p.scala:1:1: error: Not found: \xfe\n  x\n  ^\n1 error found\n")
        check("pairs: two bytes not UTF-8 kept apart", not compare(read_text(os.path.join(d, "b1.txt")), read_text(os.path.join(d, "b2.txt")))[1], False)
    # The readers' rendering (tests/scala3/run.py's rule_text, the typing harness): the layout read, the name whole.
    for msg, want in (("Not found: type Two Words", "type Two Words not found"), ("Not found: two words", "not found: two words"),
                      ("Not found: `x`", "not found: `x`"), ('Found:    "a, required B"\nRequired: Int', 'type mismatch: found "a, required B", required Int'),
                      ("Found:    List[Int\nRequired: Int", "type mismatch: found List[Int, required Int"),
                      ("value a is not a member of B", "value a is not a member of B"),
                      # a row: a term named `type Widget` read as the missing type
                      ("Not found: type Widget", "type Widget not found")):
        check(f"a reader's rendering of {msg!r}", render_message(msg, strict=False)[1] == want, True)
    print(f"rewording test: {n - bad} of {n} as they must be")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
