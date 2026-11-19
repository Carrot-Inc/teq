"""The facts of scalac's typer tree that no pickle carries (ORACLE), named in one place: tests/init-view/compare.py excuses
a hunk of lines with no partner only where it is one of these, and tests/init-view/by_kind.py files the differing lines by
these names. Each is checked on scalac's own pickle (`teq tasty --positions` or `--trees` of scalac's classes) and cited.

Lines (a line and its partner differing in place):
- POSITION: a position equal to its children's envelope, which `PositionPickler` does not write (PositionPickler.scala
  119-120: `x.span.toSynthetic != x.envelope(x.source) || alwaysNeedsPos(x)`), so the unpickled point is the start.
- PACKAGE_PREFIX: a package's prefix, `TERMREFpkg` in any pickle (TreePickler.scala 248-251).
- ENUM_ORDINAL: `Enum.ordinal` the typer chose before `SyntheticMembers` added the override (a plain `SELECT`,
  TreeUnpickler.scala 1353-1356).
- RETURN_FROM: a `Return`'s `from`, `Ident(TermRef(NoPrefix, owner))` in the typer (Typer.scala 2451) and
  `Ident(from.termRef)` read back (TreeUnpickler.scala 1577).
- LAZY_COMPLETION: a library type whose denotation scalac has not computed when its dump reads it (`currentSymbol` in
  `stopBecauseStaticOrLocal`, Types.scala 6220-6226): its package prefix visited (shown by a probe of InitDump).

Hunks (lines of one dump with no partner in the other):
- ACCESSOR_PLACE: a parameter accessor's place among a template's statements: `TreePickler` writes the parameters apart
  (`decomposeTemplateBody`, TreePickler.scala 681), `readTemplate` reads them first (`tparams ++ vparams ++ stats`,
  TreeUnpickler.scala 1172). Only whole accessors (a `paramAccessor` ValDef with its deeper lines), and only moved: the
  accessors a unit's hunks excuse on one side are those they excuse on the other.
- PACKAGE_ID: a package clause's qualified id: scalac's `Select(Ident(a), b)` against the one `TERMREFpkg` of the pickle,
  read back as `tpd.ref`'s `Ident` (TreeUnpickler.scala 1209)."""
import collections
import re

POSITION = "ORACLE: a position the pickle does not carry"
PACKAGE_PREFIX = "ORACLE: a package prefix, TERMREFpkg in any pickle"
ENUM_ORDINAL = "ORACLE: a symbol the typer chose before a synthetic override"
RETURN_FROM = "ORACLE: a Return's from, NoPrefix against from.termRef"
LAZY_COMPLETION = "ORACLE: a library type scalac has not completed (currentSymbol), its package prefix visited"
ACCESSOR_PLACE = "ORACLE: a parameter accessor's place among the statements"
PACKAGE_ID = "ORACLE: a package clause's qualified id, one TERMREFpkg"

SYM = re.compile(r" sym=(\S+)")


def _indent(l):
    return len(l) - len(l.lstrip())


def _kind(l):
    return l.strip().split(" ", 1)[0]


def accessor_groups(lines):
    """The lines as whole parameter accessors (a `paramAccessor` ValDef and the lines deeper than it), or None where a
    line is no part of one."""
    groups, base = [], None
    for l in lines:
        if _kind(l) == "ValDef" and "paramAccessor" in l:
            groups.append([l])
            base = _indent(l)
        elif groups and base is not None and _indent(l) > base:
            groups[-1].append(l)
        else:
            return None
    return groups


def package_id(a, b):
    """scalac's `Select(Ident(a), b)` of a package clause against the pickle's one `Ident` of the same package."""
    if len(a) != 2 or len(b) != 1:
        return False
    if (_kind(a[0]), _kind(a[1]), _kind(b[0])) != ("Select", "Ident", "Ident"):
        return False
    if not all("{package" in l for l in a + b):
        return False
    sa, sb = SYM.search(a[0]), SYM.search(b[0])
    return bool(sa and sb and sa.group(1) == sb.group(1))


def hunks_excused(hunks, key=lambda l: l):
    """Of a unit's hunks (pairs of scalac's lines and teq's with no partner), the ones no ORACLE kind excuses, each with
    its reason; the accessors excused on one side must be those excused on the other (moved, not lost), compared by
    `key` of their lines."""
    failing, left, right = [], collections.Counter(), collections.Counter()
    for a, b in hunks:
        if package_id(a, b):
            continue
        ga, gb = accessor_groups(a), accessor_groups(b)
        if ga is not None and gb is not None and (ga or gb):
            left.update(tuple(key(l) for l in g) for g in ga)
            right.update(tuple(key(l) for l in g) for g in gb)
            continue
        failing.append((a, b, "no ORACLE kind"))
    if left != right:
        lost = list((left - right).elements()) + list((right - left).elements())
        failing.append(([l for g in lost for l in g], [], "accessors not moved but lost or added"))
    return failing
