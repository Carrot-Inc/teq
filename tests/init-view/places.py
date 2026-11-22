#!/usr/bin/env python3
"""places.py: the table of the places the TASTy writer takes from what the parser read, each construct written with
what may stand between its keyword and its definition or just before its keyword (tests/init-view/places.sh runs it).

  places.py generate <dir>   writes one program per cell, `<construct>--<trivia>--<slot>.scala`
  places.py judge <corpus work dir> <head dir>
  places.py --self-test      the judge on planted records (`self_test`)

A construct is a place one of the writer's records gives: an extension group's `extension` (each written form), an
enum case's `case` (alone, a list, a class case), a package object's `package`, a template's body after its parents
(`extends`, `with`, `derives`, a self type alone, a colon), a file's first statement after each header form, a type
parameter's variance sign, a function type's arrow, a type selection's dot, and the dot of a path in `new`. A trivia is
what stands in one of two slots, `before` the keyword or `between` the keyword and the definition: nothing, a line
comment, a block comment and a nested one holding the keyword, a string, a triple-quoted string and an interpolated
one holding it, a backticked identifier that is it, identifiers that end and start with it, an annotation, a modifier,
another construct of the same keyword. A cell whose program scalac refuses is named, not dropped.

The constructs named `s<n>-...` stand each for one of the writer's reads of the source's text where the parser had the
token in hand (READS below, by its number `<n>`): the unit's package clause and its name, a type parameter's variance
sign, an anonymous class's braces, a type selection's name, an inferred type after a definition's name, a call of a
top-level method by its name alone, a call's closing parenthesis, the keyword `match`, the last dot of a `new`'s path,
a selection's name, a call's name and its argument's opening, a lambda's arrow, a case's keyword. Each is written plain
and with the token's neighbours varied: the trivia above where they may stand, a blank and a line break where the
grammar allows one, a backquoted name, a non-ASCII or operator name, a string holding the token, the parenthesized form.
Further constructs reach each call of the parser that consumes a read's token on a path the first ones do not (a colon's
lambda, a lambda past a block's first statement, a polymorphic literal, a sign in an alias's or an enum case's or a
higher-kinded clause, a pattern's path, a later argument clause, a colon's single case, an interpolator the file
defines or imports, ...); the `-utf8` ones put non-ASCII characters of each UTF-8 width before the token; and LAYOUTS
re-lays a construct per read with tabs for its indentation and with CR LF line ends.

`judge` reads the corpus run of tests/init-view-corpus.sh over the cells (per cell `scalac-view.dump`, the view of
scalac's own pickles, and `view.dump`, the view of the base's products) and the head's views (`<head dir>/<cell>/
view.dump` beside its exit `view.exit`, or `not-judged` holding why there is none), aligns each to scalac's as
regressions.py aligns (compare.py --positions), and prints the table: per cell its verdict, the lines scalac's on the
base and not on the head (lost: the head places them otherwise or has no line aligned to them), the lines the head
gains, and the lines neither has as scalac's (each a named row). A cell scalac refuses, or the base, by the exit the
corpus run recorded (1 with the compiler's error line on its record; any other exit not 0 is NOT JUDGED,
records.py's `judged_exit`), is named; a cell whose record is missing, empty, not what its sum says (another cell's, a line cut
off its end or out of its middle), past its bound or holding its unit twice (tests/init-view/records.py) is NOT JUDGED:
so scalac's dump is whole, and every line of it the head has no line for counts. Exits 1 when a cell loses one, when a
cell is NOT JUDGED, or when the head's records do not match the cells (a cell held twice, a cell with no record, a
record of no cell).
Then per cell each line of scalac's whose aligned line has another place on the head (the base's beside it where it is
not the head's), and each line of scalac's or the head's dump that no line of the other is aligned to (another tree: a
selection for an identifier); and per read of READS its constructs' cells, those judged and those scalac refuses, each
plain cell's such lines, and each such line of a variation that its construct's plain cell (`<construct>--nothing--
before`) has not the same way: one the plain cell has as scalac's (`variation only`), or wrong by another distance
from scalac's place (`plain too, moved otherwise`), told from the lines the plain cell has as far from scalac's
(`plain too`), the two cells' lines matched by their index where scalac's two trees are one kind for kind."""
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

# What stands in a slot. `kind` says where it may stand: `trivia` anywhere a blank may (a comment), `stmt` where a
# statement or a definition may, `mod` before a definition. `{k}` is the construct's keyword.
TRIVIA = [
    ("nothing", "trivia", ""),
    ("line-comment", "trivia", "// {k}\n"),
    ("block-comment", "trivia", "/* {k} */ "),
    ("nested-comment", "trivia", "/* a /* {k} */ b */ "),
    ("string", "stmt", 'def s1 = "{k}"\n'),
    ("triple-string", "stmt", 'def s2 = """{k}"""\n'),
    ("interpolated", "stmt", 'def s3 = s"{k} ${{1}}"\n'),
    ("backticked", "stmt", "def `{k}` = 1\n"),
    ("affixes", "stmt", "def my{w} = 1\ndef {w}x = 2\n"),
    ("annotation", "mod", '@deprecated("{k}", "1") '),
    ("modifier", "mod", "private "),
    # A blank and a line break, `blank` and `break`, which only the constructs that list them take.
    ("blank", "blank", " "),
    ("newline", "break", "\n"),
    # A comment of non-ASCII characters of each width in UTF-8, two bytes, three and four (a character outside the
    # Basic Multilingual Plane, two UTF-16 units), which only the constructs that list `utf8` take.
    ("utf8-2", "utf8", "/* é */ "),
    ("utf8-3", "utf8", "/* 中 */ "),
    ("utf8-4", "utf8", "/* 𝔸 */ "),
]

# The lines a `-utf8` construct starts with: an identifier of two- and three-byte characters and a string with a
# four-byte one, before the construct's token.
UTF8 = 'val é中 = 1\nval s0 = "é中𝔸"\n'


def fill(text, k, w):
    return text.replace("{k}", k).replace("{w}", w).replace("{{", "{").replace("}}", "}")


def indent(text, pad):
    """Each line of a slot's text at the construct's indentation; a text with no newline stays inline."""
    if "\n" not in text:
        return text
    return "".join(pad + line + "\n" for line in text.rstrip("\n").split("\n"))


# Each construct: its keyword as trivia hold it, an identifier word for the affixes, the program with its two slots
# (`<B>` before the keyword, `<I>` between it and the definition), each slot's indentation and the kinds it takes, and
# the same keyword's construct written once more before it.
CONSTRUCTS = [
    # An extension group at the top level, its methods in the file's wrapper: the wrapper's place is the group's
    # `extension` (Parsers.scala 4658, `ExtMethods` at `atSpan(start)`).
    ("ext-oneline", "extension", "extension", "<B>extension (x: Int) <I>def twice = x + x\n",
     {"B": ("", "trivia stmt"), "I": ("", "trivia mod")}, "extension (y: String) def size2 = y.length\n"),
    ("ext-braces", "extension", "extension", "<B>extension (x: Int) {\n<I>  def twice = x + x\n}\n",
     {"B": ("", "trivia stmt"), "I": ("  ", "trivia stmt mod")}, "extension (y: String) def size2 = y.length\n"),
    ("ext-colon", "extension", "extension", "<B>extension (x: Int):\n<I>  def twice = x + x\n",
     {"B": ("", "trivia stmt"), "I": ("  ", "trivia stmt mod")}, "extension (y: String) def size2 = y.length\n"),
    ("ext-indented", "extension", "extension", "<B>extension (x: Int)\n<I>  def twice = x + x\n",
     {"B": ("", "trivia stmt"), "I": ("  ", "trivia stmt mod")}, "extension (y: String) def size2 = y.length\n"),
    ("ext-collective", "extension", "extension",
     "<B>extension [T](xs: List[T])\n<I>  def second: T = xs(1)\n  def third: T = xs(2)\n",
     {"B": ("", "trivia stmt"), "I": ("  ", "trivia stmt mod")}, "extension (y: String) def size2 = y.length\n"),
    ("ext-using", "extension", "extension",
     "<B>extension (x: Int)(using o: Ordering[Int])\n<I>  def max2(y: Int): Int = o.max(x, y)\n",
     {"B": ("", "trivia stmt"), "I": ("  ", "trivia stmt mod")}, "extension (y: String) def size2 = y.length\n"),
    # An enum case from its `case` where it is alone (Desugar's enum case at the `case`), a list's from its name.
    ("enum-case", "case", "case", "enum Color:\n<B>  case <I>Red\n  case Green\n",
     {"B": ("  ", "trivia stmt mod"), "I": ("", "trivia")}, "case Blue\n"),
    ("enum-case-list", "case", "case", "enum Color:\n<B>  case <I>Red, Green, Blue\n",
     {"B": ("  ", "trivia stmt mod"), "I": ("", "trivia")}, "case Black\n"),
    ("enum-class-case", "case", "case", "enum Shape:\n<B>  case <I>Circle(r: Double)\n  case Square(s: Double)\n",
     {"B": ("  ", "trivia stmt mod"), "I": ("", "trivia")}, "case Dot\n"),
    # A package object's package at its `package` (`package object p` is `package p { object package }`).
    ("package-object", "package", "package", "<B>package <I>object pkgo {\n  def f = 1\n}\n",
     {"B": ("", "trivia stmt"), "I": ("", "trivia")}, "package object other {\n  def g = 2\n}\n"),
    # A template's self definition from the body's first token, after its parents (`Parsers.selfType`).
    ("template-extends", "{", "body", "trait B\ntrait C\ntrait A extends B <B>{<I> self: C =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-with", "{", "body", "trait B\ntrait C\ntrait D\ntrait A extends B with D <B>{<I> self: C =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-derives", "{", "body", "case class A(x: Int) derives CanEqual <B>{<I> self =>\n  def f = x\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-self", "{", "body", "trait C\ntrait A <B>{<I> this: C =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-colon", ":", "body", "trait B\ntrait C\ntrait A extends B<B>:\n<I>  self: C =>\n  def f = 1\n",
     {"B": ("", "trivia"), "I": ("  ", "trivia")}, ""),
    # The same construct nested in it and before it: each template's self definition its own, past the bodies nested
    # in it (the parser hands a body's self start to its class once the body ends).
    ("template-nested", "{", "body",
     "trait C\ntrait D\ntrait A <B>{<I> self: C =>\n  trait N { inner: D =>\n    def g = 1\n  }\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-nested-plain", "{", "body",
     "trait C\ntrait D\ntrait A <B>{<I>\n  trait N { inner: D =>\n    def g = 1\n  }\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-anonymous", "{", "body", "trait B\nval x = new B <B>{<I> me =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-after-given", "{", "body",
     "trait C\ngiven g: C with { me =>\n  def h = 1\n}\ntrait A extends C <B>{<I> self: C =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    # `val a, b = e` reads `e` once for each name (the parser's rewind).
    ("template-rewound", "{", "body", "trait B\nval a, b = new B <B>{<I> me =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("template-parent-brace", "{", "body",
     "class P(s: String)\ntrait C\nclass A extends P(\"{\") <B>{<I> self: C =>\n  def f = 1\n}\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    # A file's first statement, the empty package's identifier where its statements are the file's (Parsers.scala 5050
    # `compilationUnit`: past leading `;`, the first statement's start; SourceFile.scala 27-37 a script's header).
    ("first-import", "import", "import", "<B>import scala.Int\nval x: Int = 1\n",
     {"B": ("", "trivia stmt mod header")}, ""),
    ("first-def", "def", "def", "<B>def f = 1\nval x = f\n", {"B": ("", "trivia stmt mod header")}, ""),
    ("first-object", "object", "object", "<B>object O:\n  def f = 1\nval x = O.f\n",
     {"B": ("", "trivia stmt mod header")}, ""),
    # A variant type parameter from its sign (`Parsers.typeParam`'s `atSpan(start, nameStart)`).
    ("variance", "+", "plus", "class Box[<B>+<I>A, -B](a: A)\n", {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    # A parenthesized function type's arrow (its `Function` at the arrow), a dependent one's start.
    ("arrow", "=>", "arrow", "type F = (Int) <B>=><I> Int\n", {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("arrow-context", "?=>", "arrow", "type G = (Int) <B>?=><I> Int\n", {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    # A type selection's point at its name (`Parsers.selector`), the path of a `new` the same.
    ("selection", ".", "dot", "val xs: scala.collection.immutable<B>.<I>List[Int] = Nil\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("new-path", ".", "dot", "object P:\n  class C\nval c = new P<B>.<I>C\n", {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    # The writer's reads of the text (READS), each construct named by its read's number. The unit's `PackageDef` from
    # its first package clause's `package` to its end, its point the clause's last name (`Parsers.compilationUnit`,
    # `makePackaging`'s `atSpan(start, pointOffset(pkg))`).
    ("s01-package", "package", "package", "<B>package <I>pa.pb\n\nval x = 1\n",
     {"B": ("", "trivia header mod"), "I": ("", "trivia blank break")}, ""),
    ("s01-package-dot", ".", "dot", "package pa<B>.<I>pb\n\nval x = 1\n",
     {"B": ("", "trivia blank break"), "I": ("", "trivia blank break")}, ""),
    ("s01-package-backquoted", "package", "package", "<B>package <I>pa.`pb`\n\nval x = 1\n",
     {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    ("s01-package-colon", ":", "colon", "<B>package pa.pb<I>:\n  val x = 1\n", {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    # A class's type parameter from its variance sign, its point its name (`Parsers.typeParam`'s `atSpan(start,
    # nameStart)`): the sign before the name, and the point after the sign.
    ("s02-variance-plus", "+", "plus", "class Box[<B>+<I>A](a: A)\n", {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s02-variance-minus", "-", "minus", "class Sink[<B>-<I>A]:\n  def put(a: A): Unit = ()\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s02-variance-annotated", "+", "plus", "class Box[@deprecated <B>+<I>A](a: A)\n",
     {"B": ("", "plain"), "I": ("", "trivia blank break")}, ""),
    # A type parameter whose name starts with a sign (`++`, an operator name, invariant): its point its name's start,
    # where the writer looks for a sign at its start.
    ("s03-sign-named", "+", "plus", "class Box[<B>++](a: ++)\n", {"B": ("", "trivia")}, ""),
    ("s03-sign-named-method", "-", "minus", "def f[<B>--](x: --) = x\n", {"B": ("", "trivia")}, ""),
    ("s03-sign-named-variant", "+", "plus", "class Box[+ <I>++](a: ++)\n", {"B": ("", "plain"), "I": ("", "trivia")}, ""),
    # An anonymous class's template in braces from `new` through `}` (`Parsers.newExpr`'s `reposition`): the `{`
    # after its last parent, or after `new` where it has none, and the `}` it ends with.
    ("s04-anon", "{", "brace", "trait B\nval x = new B <B>{\n  def f = 1\n}\n", {"B": ("", "trivia blank break")}, ""),
    ("s04-anon-with", "{", "brace", "trait B\ntrait C\nval x = new B with C <B>{\n  def f = 1\n}\n",
     {"B": ("", "trivia blank break")}, ""),
    ("s04-anon-args", "{", "brace", "class P(s: String)\nval x = new P(\"{\") <B>{\n  def f = 1\n}\n",
     {"B": ("", "trivia blank break")}, ""),
    ("s04-anon-bare", "{", "brace", "val x = new <B>{\n  def f = 1\n}\n", {"B": ("", "trivia blank break")}, ""),
    ("s04-anon-typeargs", "{", "brace", "trait B[T]\nval x = new B[Int] <B>{\n  def f = 1\n}\n", {"B": ("", "trivia")}, ""),
    ("s04-anon-end", "}", "brace", "trait B\nval x = new B {\n  def f = 1\n<B>}\n", {"B": ("", "trivia")}, ""),
    # A type selection's point at its name (`Parsers.selector`), in each place the writer reads it: a val's type, an
    # alias's right-hand side, a parameter's type, a result type, an ascription, a typed pattern.
    ("s05-val-type", ".", "dot", "object P:\n  type T = Int\nval x: P<B>.<I>T = 1\n",
     {"B": ("", "trivia blank break"), "I": ("", "trivia blank break")}, ""),
    ("s05-val-type-backquoted", ".", "dot", "object P:\n  type T = Int\nval x: P<B>.<I>`T` = 1\n",
     {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    ("s05-alias-rhs", ".", "dot", "object P:\n  type T = Int\ntype U = P<B>.<I>T\n",
     {"B": ("", "plain"), "I": ("", "trivia blank break")}, ""),
    ("s05-param-type", ".", "dot", "object P:\n  type T = Int\ndef f(x: P<B>.<I>T) = x\n",
     {"B": ("", "plain"), "I": ("", "trivia blank break")}, ""),
    ("s05-result-type", ".", "dot", "object P:\n  type T = Int\ndef h: P<B>.<I>T = 1\n",
     {"B": ("", "plain"), "I": ("", "trivia blank break")}, ""),
    ("s05-ascription", ".", "dot", "object P:\n  type T = Int\nval y = (1: P<B>.<I>T)\n",
     {"B": ("", "plain"), "I": ("", "trivia blank break")}, ""),
    ("s05-typed-pattern", ".", "dot", "object P:\n  class C\ndef g(v: Any) = v match\n  case c: P<B>.<I>C => 1\n  case _ => 2\n",
     {"B": ("", "plain"), "I": ("", "trivia blank break")}, ""),
    # A definition's inferred type, the empty span after its name (`Parsers.typedOpt`'s `Span(in.lastOffset)`): the
    # name's end, a backquoted one's closing quote.
    ("s06-val", "=", "eq", "<B>val x<I> = 1\n", {"B": ("", "plain"), "I": ("", "trivia blank break")}, ""),
    ("s06-val-backquoted", "`", "quote", "<B>val `x y`<I> = 1\n", {"B": ("", "plain"), "I": ("", "trivia blank break")}, ""),
    ("s06-def-backquoted", "`", "quote", "<B>def `f g`<I> = 1\n", {"B": ("", "plain"), "I": ("", "trivia blank break")}, ""),
    ("s06-val-operator", "=", "eq", "<B>val +++<I> = 1\n", {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    ("s06-def-unicode", "=", "eq", "<B>def é<I> = 1\n", {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    # A call of a top-level method of the unit by its name alone, an identifier as scalac's typer keeps it, against a
    # selection on the file's wrapper: the writer reads the call's text for the name.
    ("s07-call", "f", "f", "def f(x: Int) = x\nval y = <B>f<I>(1)\n", {"B": ("", "trivia"), "I": ("", "trivia blank")}, ""),
    ("s07-call-backquoted", "f", "f", "def f(x: Int) = x\nval y = <B>`f`(1)\n", {"B": ("", "trivia")}, ""),
    ("s07-call-parenthesized", "f", "f", "def f(x: Int) = x\nval y = <B>(f)(1)\n", {"B": ("", "trivia")}, ""),
    ("s07-call-prefix", "f", "f", "def f(x: Int) = x\ndef fx(x: Int) = x\nval y = <B>fx(1) + f(2)\n", {"B": ("", "trivia")}, ""),
    ("s07-call-typeargs", "f", "f", "def f[T](x: T) = x\nval y = <B>f[Int](1)\n", {"B": ("", "trivia")}, ""),
    ("s07-call-suffix", "f", "f", "def f(x: Int) = x\ndef f1(x: Int) = x\ndef f$(x: Int) = x\nval y = <B>f1(1) + f$(2) + f(3)\n",
     {"B": ("", "trivia")}, ""),
    # A call of a Java-defined member or one of `Any` whose one empty argument clause the source leaves out (the
    # selection spans the call): the writer tells it by the call's last byte, `)`.
    ("s08-length", ")", "paren", "val n = \"a\".length<B>\n", {"B": ("", "trivia")}, ""),
    ("s08-length-applied", ")", "paren", "val n = \"a\".length<B>()<I>\n", {"B": ("", "trivia blank"), "I": ("", "trivia")}, ""),
    ("s08-length-parenthesized", ")", "paren", "val n = (\"a\".length)<B>\n", {"B": ("", "trivia")}, ""),
    ("s08-tostring", ")", "paren", "def t(a: Any) = a.toString<B>\n", {"B": ("", "trivia")}, ""),
    ("s08-tostring-parenthesized", ")", "paren", "def t(a: Any) = (a.toString)<B>\n", {"B": ("", "trivia")}, ""),
    ("s08-hashcode-applied", ")", "paren", "def h(a: Any) = a.hashCode<B>()\n", {"B": ("", "trivia blank")}, ""),
    # A match's point at its keyword after the scrutinee (`Parsers.matchClause`'s `atSpan(startOffset(t),
    # in.skipToken())`), a selected one's (`x.match`) the same.
    ("s09-match", "match", "match", "def f(x: Int) = x<B> match\n  case 1 => \"one\"\n  case _ => \"other\"\n",
     {"B": ("", "trivia blank break")}, ""),
    ("s09-match-parenthesized", "match", "match", "def f(x: Int) = (x)<B> match\n  case 1 => \"one\"\n  case _ => \"other\"\n",
     {"B": ("", "trivia")}, ""),
    ("s09-match-braces", "match", "match", "def f(x: Int) = x<B> match { case 1 => \"one\" case _ => \"other\" }\n",
     {"B": ("", "trivia blank break")}, ""),
    ("s09-match-selected", "match", "match", "def f(x: Int) = x<B>.<I>match { case 1 => \"one\" case _ => \"other\" }\n",
     {"B": ("", "trivia blank"), "I": ("", "trivia blank")}, ""),
    ("s09-match-string", "match", "match", "def f = \"match\"<B> match\n  case \"a\" => 1\n  case _ => 2\n", {"B": ("", "trivia")}, ""),
    ("s09-match-call", "match", "match",
     "def g(x: Int) = x\ndef f(x: Int) = g(x)<B> match\n  case 1 => \"one\"\n  case _ => \"other\"\n", {"B": ("", "trivia")}, ""),
    # A `new`'s type tree's point at its path's last name (`Select`'s): the writer finds it by the path's last dot.
    ("s10-new", ".", "dot", "object P:\n  class C\nval c = new P<B>.<I>C\n", {"B": ("", "plain blank break"), "I": ("", "blank break")}, ""),
    ("s10-new-backquoted", ".", "dot", "object P:\n  class C\nval c = new P<B>.<I>`C`\n",
     {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    ("s10-new-unicode", ".", "dot", "object P:\n  class Cé\nval c = new P<B>.<I>Cé\n", {"B": ("", "plain"), "I": ("", "trivia")}, ""),
    ("s10-new-operator", ".", "dot", "object P:\n  class +++\nval c = new P<B>.<I>+++\n", {"B": ("", "plain"), "I": ("", "blank")}, ""),
    ("s10-new-args", ".", "dot", "object P:\n  class D(i: Int)\nval d = new P<B>.<I>D(1)\n",
     {"B": ("", "trivia blank"), "I": ("", "trivia blank")}, ""),
    ("s10-new-deep", ".", "dot", "object P:\n  object Q:\n    class C\nval c = new P.Q<B>.<I>C\n",
     {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    ("s10-new-typeargs", ".", "dot", "object P:\n  class E[T]\nval e = new P<B>.<I>E[Int]\n", {"B": ("", "plain"), "I": ("", "trivia")}, ""),
    # A value's selection's point at its name (`Parsers.selector`): the name at the selection's end, or backquoted.
    ("s11-field", ".", "dot", "object O:\n  val x = 1\nval y = O<B>.<I>x\n",
     {"B": ("", "trivia blank break"), "I": ("", "trivia blank break")}, ""),
    ("s11-field-backquoted", ".", "dot", "object O:\n  val x = 1\nval y = O<B>.<I>`x`\n",
     {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    ("s11-field-this", ".", "dot", "class K:\n  val x = 1\n  def g = this<B>.<I>x\n", {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    ("s11-field-unicode", ".", "dot", "object O:\n  val é = 1\nval y = O<B>.<I>é\n", {"B": ("", "plain"), "I": ("", "trivia")}, ""),
    ("s11-field-operator", ".", "dot", "object O:\n  val +++ = 1\nval y = O<B>.<I>+++\n", {"B": ("", "plain"), "I": ("", "blank")}, ""),
    ("s11-field-postfix", "x", "x", "import scala.language.postfixOps\nobject O:\n  val x = 1\nval y = O <B>x\n", {"B": ("", "trivia")}, ""),
    # A call's function's point at its name (`Parsers.selector`), its application's at the argument clause's opening
    # (`simpleExprRest`'s `atSpan(startOffset(t), in.offset)`), an infix operator's selection at the operator.
    ("s12-call", ".", "dot", "object O:\n  def m(a: Int) = a\nval y = O<B>.<I>m(1)\n",
     {"B": ("", "trivia blank break"), "I": ("", "trivia blank break")}, ""),
    ("s12-call-backquoted", ".", "dot", "object O:\n  def m(a: Int) = a\nval y = O<B>.<I>`m`(1)\n",
     {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    ("s12-call-open", "(", "paren", "object O:\n  def m(a: Int) = a\nval y = O.m<B>(1)\n", {"B": ("", "trivia blank")}, ""),
    ("s12-call-braces", "{", "brace", "object O:\n  def m(a: Int) = a\nval y = O.m<B>{ 1 }\n", {"B": ("", "trivia blank")}, ""),
    ("s12-call-typeargs", ".", "dot", "object O:\n  def p[T](a: T) = a\nval y = O<B>.<I>p[Int](1)\n",
     {"B": ("", "plain"), "I": ("", "trivia")}, ""),
    ("s12-nullary", ".", "dot", "object O:\n  def n = 1\nval y = O<B>.<I>n\n", {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    ("s12-nullary-postfix", "n", "n", "import scala.language.postfixOps\nobject O:\n  def n = 1\nval y = O <B>n\n", {"B": ("", "trivia")}, ""),
    ("s12-infix", "plus", "plus", "class N(v: Int):\n  def plus(o: Int) = v + o\ndef y(n: N) = n <B>plus<I> 2\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("s12-infix-backquoted", "plus", "plus", "class N(v: Int):\n  def plus(o: Int) = v + o\ndef y(n: N) = n <B>`plus` 2\n",
     {"B": ("", "trivia")}, ""),
    # A lambda's point at its arrow (`Parsers.closureRest`'s, `funParams`'), a placeholder's parameter at its `_`.
    ("s13-lambda", "=>", "arrow", "val f = (x: Int)<B> =><I> x + 1\n", {"B": ("", "trivia blank break"), "I": ("", "trivia blank break")}, ""),
    ("s13-lambda-untyped", "=>", "arrow", "val f: Int => Int = x<B> =><I> x + 1\n",
     {"B": ("", "trivia blank break"), "I": ("", "trivia")}, ""),
    ("s13-lambda-unicode", "⇒", "arrow", "val f = (x: Int)<B> ⇒<I> x + 1\n", {"B": ("", "trivia"), "I": ("", "blank")}, ""),
    ("s13-lambda-two", "=>", "arrow", "val f = (x: Int, y: Int)<B> => x + y\n", {"B": ("", "trivia")}, ""),
    ("s13-lambda-context", "?=>", "arrow", "val g: Int ?=> Int = (x: Int)<B> ?<I>=> x\n", {"B": ("", "trivia blank"), "I": ("", "blank")}, ""),
    ("s13-lambda-literal-type", "=>", "arrow", "val f = (x: \"=>\")<B> => x\n", {"B": ("", "trivia")}, ""),
    ("s13-lambda-function-type", "=>", "arrow", "val f = (g: Int => Int)<B> => g(1)\n", {"B": ("", "trivia")}, ""),
    ("s13-lambda-block", "=>", "arrow", "val ys = List(1).map { x<B> => x + 1 }\n", {"B": ("", "trivia break")}, ""),
    ("s13-placeholder", "=>", "arrow", "val h = (_: Int)<B> + 1\n", {"B": ("", "trivia")}, ""),
    # A case clause from its keyword (`Parsers.caseClause`'s `atSpan(in.offset)`): the writer walks back from the
    # pattern over blanks to it.
    ("s14-case", "case", "case", "<B>def f(x: Int) = x match\n  case<I> 1 => \"one\"\n  case _ => \"other\"\n",
     {"B": ("", "plain"), "I": ("", "trivia blank break")}, ""),
    ("s14-case-parenthesized", "case", "case", "<B>def f(x: Int) = x match\n  case<I> (1) => \"one\"\n  case _ => \"other\"\n",
     {"B": ("", "plain"), "I": ("", "trivia")}, ""),
    ("s14-case-backquoted", "case", "case", "<B>def f(x: Int, y: Int) = x match\n  case<I> `y` => \"y\"\n  case _ => \"other\"\n",
     {"B": ("", "plain"), "I": ("", "trivia")}, ""),
    ("s14-catch", "case", "case", "<B>def f = try 1 catch\n  case<I> e: Exception => 2\n", {"B": ("", "plain"), "I": ("", "trivia")}, ""),
    ("s14-partial", "case", "case", "<B>val pf: PartialFunction[Int, Int] = {\n  case<I> 1 => 2\n}\n",
     {"B": ("", "plain"), "I": ("", "trivia")}, ""),
    ("s14-case-oneline", "case", "case", "<B>def f(x: Int) = x match { case 1 => \"a\" case<I> _ => \"b\" }\n",
     {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    # Further constructs of the reads, each reaching a call of the parser that consumes the read's token and that none
    # above reaches. The unit's second leading package clause, a `PackageDef` of its own inside the first's.
    ("s01-package-two", "package", "package", "<B>package pa\npackage <I>pb\n\nval x = 1\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    # A sign in the other clauses that take one: an enum's class case's (`parse_enum_cases`), an alias's (`typeDefOrDcl`'s
    # `ParamOwner.Hk`), a higher-kinded parameter's own parameter's (`parse_hk_clause`); and a member class's.
    ("s02-enum-case-variance", "+", "plus", "enum Opt[+T]:\n  case Some1[<B>+<I>U](u: U) extends Opt[U]\n  case None1 extends Opt[Nothing]\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s02-variance-member", "+", "plus", "object W:\n  class Box[<B>+<I>A](a: A)\n", {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s03-alias-variance", "+", "plus", "type F[<B>+<I>X] = List[X]\n", {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s03-hk-variance", "+", "plus", "class C[F[<B>+<I>X]]\n", {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    # An anonymous class whose template follows a colon, which the reader of a braced one consumes on the same path; a
    # member's braced one.
    ("s04-anon-colon", "{", "brace", "trait B\nval x = new B:<B>\n  def f = 1\n", {"B": ("", "trivia")}, ""),
    ("s04-anon-member", "{", "brace", "trait B\nobject W:\n  val x = new B <B>{\n    def f = 1\n  }\n", {"B": ("", "trivia blank break")}, ""),
    # A match whose scrutinee is a block, which continues the block (`parse_block_tail`); a member's match.
    ("s09-match-block", "match", "match", "def f(x: Int) = { x }<B> match\n  case 1 => \"one\"\n  case _ => \"other\"\n",
     {"B": ("", "trivia")}, ""),
    ("s09-match-member", "match", "match", "object W:\n  def f(x: Int) = x<B> match\n    case 1 => \"one\"\n    case _ => \"other\"\n",
     {"B": ("", "trivia blank break")}, ""),
    # An annotation's class path (`parse_class_path`), the type of its `New`.
    ("s10-annotation-path", ".", "dot", "object P:\n  class a extends scala.annotation.StaticAnnotation\n@P<B>.<I>a def f = 1\n",
     {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    # A pattern's stable path (`parse_simple_pattern`).
    ("s11-pattern-path", ".", "dot", "object O:\n  val x = 1\ndef g(v: Int) = v match\n  case O<B>.<I>x => 1\n  case _ => 2\n",
     {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    # A call's later argument clause: a curried call's, a `new`'s second (an application of the instance), an
    # annotation's second; and a colon's argument on the next line, whose application's point is that line's first
    # token (`argumentStart`, `simpleExprRest`'s `in.offset` at the indent).
    ("s12-call-curried", "(", "paren", "def f(a: Int)(b: Int) = a + b\nval y = f(1)<B>(2)\n", {"B": ("", "trivia blank")}, ""),
    ("s12-new-second-args", "(", "paren", "class C(a: Int)(b: Int)\nval c = new C(1)<B>(2)\n", {"B": ("", "trivia blank")}, ""),
    ("s12-annotation-args", "(", "paren", "class an(a: Int)(b: Int) extends scala.annotation.StaticAnnotation\n@an(1)<B>(2) def f = 1\n",
     {"B": ("", "trivia blank")}, ""),
    ("s12-call-colon", ":", "colon", "object O:\n  def m(a: Int) = a\nval y = O.m<B>:<I>\n  1\n", {"B": ("", "plain blank"), "I": ("", "trivia")}, ""),
    # A lambda past a block's first statement and a case body's (`parse_block_lambda`); a polymorphic literal's own
    # arrow (`Parsers.expr`'s `accept(ARROW)`, the literal's point) and its function literal's.
    ("s13-lambda-block-later", "=>", "arrow", "val f: Int => Int = {\n  val k = 1\n  x<B> =><I> x + k\n}\n",
     {"B": ("", "trivia blank break"), "I": ("", "trivia blank break")}, ""),
    ("s13-lambda-case-body", "=>", "arrow", "def g(v: Int): Int => Int = v match\n  case _ => x<B> =><I> x + v\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank")}, ""),
    ("s13-poly", "=>", "arrow", "val p = [T]<B> =><I> (x: T) => x\n", {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("s13-poly-inner", "=>", "arrow", "val p = [T] => (x: T)<B> =><I> x\n", {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    # A polymorphic literal's context bound, whose evidence the parser makes a context function of (`elimContextBounds`,
    # `poly_lambda_evidence`): around the body, and, where a parameter's type names the evidence, outside the written
    # function literal, which then is a node of its own.
    ("s13-poly-bound", "=>", "arrow", "val p = [T: Ordering] => (x: T)<B> =><I> x\n", {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    ("s13-poly-bound-named", "=>", "arrow", "val p = [T: Ordering as ord] => (x: T, o: ord.type)<B> =><I> x\n",
     {"B": ("", "trivia"), "I": ("", "trivia")}, ""),
    # An inline match, the match the parser rewrites in place (Parsers.scala's `InlineMatch(..).withSpan(t.span)`).
    ("s09-inline-match", "match", "match", "inline def f(x: Int): String = inline x<B> match\n  case 1 => \"one\"\n  case _ => \"other\"\n",
     {"B": ("", "trivia blank break")}, ""),
    # A colon's lambda (`parse_colon_lambda`, `Parsers.followingIsLambdaAfterColon`), its application's point at the
    # colon (`simpleExprRest`'s `in.skipToken()`): an untyped parameter, a typed one, parameters in parentheses, nested
    # ones (the outer arrow and the inner), one inside an argument list, a body of statements, a body on the arrow's
    # line (`relaxedLambdaSyntax`, which a file's import enables), an operator's (`infixOps`); and a colon's argument
    # whose lambda starts the next line, a block's first statement (`parse_block_rest_scoped`).
    ("s13-colon", "=>", "arrow", "val xs = List(1, 2)\nval ys = xs.map: x<B> =><I>\n  x + 1\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s13-colon-typed", "=>", "arrow", "val xs = List(1, 2)\nval ys = xs.map: (x: Int)<B> =><I>\n  x + 1\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s13-colon-parens", "=>", "arrow", "val xs = List(1, 2)\nval s = xs.foldLeft(0): (acc, x)<B> =><I>\n  acc + x\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s13-colon-nested", "=>", "arrow", "val xs = List(1, 2)\nval zs = xs.flatMap: x<B> =><I>\n  xs.map: y =>\n    x + y\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s13-colon-nested-inner", "=>", "arrow", "val xs = List(1, 2)\nval zs = xs.flatMap: x =>\n  xs.map: y<B> =><I>\n    x + y\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s13-colon-in-args", "=>", "arrow", "val xs = List(1, 2)\nval n = List(xs.map: x<B> =><I>\n    x + 1\n  )\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s13-colon-block", "=>", "arrow", "val xs = List(1, 2)\nval ys = xs.map: x<B> =><I>\n  val y = x + 1\n  y * 2\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s13-colon-oneline", "=>", "arrow",
     "import scala.language.experimental.relaxedLambdaSyntax\nval xs = List(1, 2)\nval ys = xs.map: x<B> =><I> x + 1\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s13-colon-infix", "=>", "arrow", "val xs = List(1, 2)\nval ys = xs map: x<B> =><I>\n  x + 1\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    ("s13-colon-next-line", "=>", "arrow", "val xs = List(1, 2)\nval ys = xs.map:\n  x<B> =><I>\n    x + 1\n",
     {"B": ("", "trivia"), "I": ("", "trivia blank break")}, ""),
    # A colon's single case (`parse_single_case_lambda`, `relaxedLambdaSyntax`).
    ("s14-colon-case", "case", "case",
     "import scala.language.experimental.relaxedLambdaSyntax\n<B>val xs = List(1, 2)\nval ys = xs.map: case<I> x => x + 1\n",
     {"B": ("", "plain"), "I": ("", "trivia blank")}, ""),
    # The reads with non-ASCII characters before the token: the lines `UTF8` holds (a file's package clause, which
    # nothing but comments may precede, a comment and its names), and a comment of each width just before it.
    ("s01-package-utf8", "package", "package", "// é中𝔸\n<B>package <I>pé.p中\n\nval s0 = \"é中𝔸\"\n",
     {"B": ("", "plain utf8"), "I": ("", "utf8")}, ""),
    ("s02-variance-utf8", "+", "plus", UTF8 + "class Box[<B>+<I>A](a: A)\n", {"B": ("", "plain utf8"), "I": ("", "utf8")}, ""),
    ("s04-anon-utf8", "{", "brace", UTF8 + "trait B\nval x = new B <B>{\n  def f = 1\n}\n", {"B": ("", "plain utf8")}, ""),
    ("s05-val-type-utf8", ".", "dot", UTF8 + "object P:\n  type T = Int\nval x: P<B>.<I>T = 1\n",
     {"B": ("", "plain utf8"), "I": ("", "utf8")}, ""),
    ("s09-match-utf8", "match", "match", UTF8 + "def f(x: Int) = x<B> match\n  case 1 => \"é\"\n  case _ => \"中\"\n",
     {"B": ("", "plain utf8")}, ""),
    ("s10-new-utf8", ".", "dot", UTF8 + "object P:\n  class C\nval c = new P<B>.<I>C\n", {"B": ("", "plain utf8"), "I": ("", "utf8")}, ""),
    ("s11-field-utf8", ".", "dot", UTF8 + "object O:\n  val x = 1\nval y = O<B>.<I>x\n", {"B": ("", "plain utf8"), "I": ("", "utf8")}, ""),
    ("s12-call-utf8", "(", "paren", UTF8 + "object O:\n  def m(a: Int) = a\nval y = O.<I>m<B>(1)\n",
     {"B": ("", "plain utf8"), "I": ("", "utf8")}, ""),
    ("s13-lambda-utf8", "=>", "arrow", UTF8 + "val f = (x: Int)<B> =><I> x + 1\n", {"B": ("", "plain utf8"), "I": ("", "utf8")}, ""),
    ("s13-colon-utf8", "=>", "arrow", UTF8 + "val xs = List(1, 2)\nval ys = xs.map: x<B> =><I>\n  x + 1\n",
     {"B": ("", "plain utf8"), "I": ("", "utf8")}, ""),
    ("s14-case-utf8", "case", "case", UTF8 + "<B>def f(x: Int) = x match\n  case<I> 1 => \"é\"\n  case _ => \"中\"\n",
     {"B": ("", "plain"), "I": ("", "utf8")}, ""),
    # An interpolation whose interpolator is an extension of `StringContext` the file defines at its top level, which
    # scalac's typer applies by its name alone (`Parsers.interpolatedString`'s identifier, an `Ident`), and one an
    # import brings from an object. Nothing may stand between the interpolator and its string; before it, what a
    # blank may.
    ("s07-interp", "show", "show",
     "extension (sc: StringContext)\n  def show(args: Any*): String = sc.parts.mkString(\"|\")\nval y = <B>show\"a ${1} b\"\n",
     {"B": ("", "trivia blank break")}, ""),
    ("s07-interp-imported", "show", "show",
     "object Shows:\n  extension (sc: StringContext)\n    def show(args: Any*): String = sc.parts.mkString(\"|\")\nimport Shows.*\n"
     "val y = <B>show\"a ${1} b\"\n",
     {"B": ("", "trivia blank break")}, ""),
]

# Whole programs laid out otherwise: each line's indentation in tabs, one for every two spaces and one for an odd last
# space, so that no line mixes the two (`tab`), and every line ended with CR LF (`crlf`). Each applies, after the trivia are filled, to a construct's plain cell and to the variation
# named here, which holds a line break, so that no line is left in the other form: the cells
# `<construct>--<layout>--before` and `<construct>--<layout>-<trivia>--<slot>`. A construct whose token stands on an
# indented line or after one per read; a file's first package clause has nothing but comments before it to indent.
LAYOUTS = [
    ("s01-package", "line-comment--before", ("crlf",)),
    ("s02-variance-member", "line-comment--before", ("tab", "crlf")),
    ("s04-anon-member", "line-comment--before", ("tab", "crlf")),
    ("s05-typed-pattern", "line-comment--between", ("tab", "crlf")),
    ("s09-match-member", "line-comment--before", ("tab", "crlf")),
    ("s10-new", "newline--before", ("tab", "crlf")),
    ("s11-field-this", "line-comment--between", ("tab", "crlf")),
    ("s12-call", "line-comment--before", ("tab", "crlf")),
    ("s13-colon-nested-inner", "line-comment--before", ("tab", "crlf")),
    ("s14-case", "line-comment--between", ("tab", "crlf")),
]


def laid_out(src, layout):
    """The program `src` in the layout `tab` or `crlf`."""
    if layout == "crlf":
        return src.replace("\n", "\r\n")
    lines = []
    for line in src.split("\n"):
        pad = len(line) - len(line.lstrip(" "))
        lines.append("\t" * ((pad + 1) // 2) + line[pad:])
    return "\n".join(lines)

# The writer's reads of the source's text (src/tasty/write) that the `s<n>-` constructs exercise, by their number: the
# function and the kinds of the view's lines whose places it gives. Some constructs above them exercise one too
# (EARLIER).
READS = {
    1: ("pickle.rs root_span, first_package_clause", "PackageDef"),
    2: ("pickle.rs class_tparam_spans", "TypeDef"),
    3: ("pickle.rs type_param_def", "TypeDef"),
    4: ("pickle.rs braced_anon_span", "Template"),
    5: ("pickle.rs written_ty_pos", "Tpt"),
    6: ("pickle.rs name_end", "Tpt"),
    7: ("term.rs text_names", "Ident"),
    8: ("term.rs mark_auto_applied", "Select"),
    9: ("term.rs match_point", "Match"),
    10: ("term.rs path_type_point", "Tpt"),
    11: ("term.rs name_point", "Select"),
    12: ("term.rs call_layout_at", "Select Apply"),
    13: ("term.rs lambda_layout_at", "DefDef Closure ValDef"),
    14: ("term.rs case_layout", "CaseDef"),
}
EARLIER = {"variance": (2, 3), "template-anonymous": (4,), "selection": (5,), "new-path": (10,)}
# The `s<n>-` constructs that exercise another read beside their own: a colon's lambda is its application's argument,
# the application's point the colon (read 12's).
ALSO = {x[0]: (12,) for x in CONSTRUCTS if x[0].startswith(("s13-colon", "s14-colon"))}


def reads_of(construct):
    """The reads of READS a construct exercises: a variance sign's construct both of its reads."""
    if construct in EARLIER:
        return EARLIER[construct]
    if construct.startswith("s") and construct[1:3].isdigit() and construct[3:4] == "-":
        n = int(construct[1:3])
        return ((2, 3) if n == 2 else (n,)) + ALSO.get(construct, ())
    return ()

# A script's header forms (SourceFile.scala 27-37: a `#!` line, or through a line `!#` or `::!#`), and leading `;`,
# which only a file's first statement has before it.
HEADERS = [
    ("header-hash-bang", "#!/bin/sh\n"),
    ("header-bang-hash", "#!/bin/sh\nexec scala \"$0\"\n!#\n"),
    ("header-colon-bang-hash", "#!/bin/sh\nexec scala \"$0\"\n::!#\n"),
    ("leading-semicolon", ";\n"),
    ("leading-semicolons", ";;\n\n"),
]


def cells():
    """Every cell: (name, program)."""
    out = []
    for name, k, w, program, slots, again in CONSTRUCTS:
        for slot, (pad, kinds) in slots.items():
            kinds = kinds.split()
            # `plain`: the slot's plain cell alone, nothing in it.
            trivia = [(t, kind, text) for t, kind, text in TRIVIA if kind in kinds or (t == "nothing" and "plain" in kinds)]
            if "stmt" in kinds and again:
                trivia.append(("same-construct", "stmt", again))
            marker = "<B>" if slot == "B" else "<I>"
            before = program.split(marker)[0].replace("<B>", "").replace("<I>", "")
            inline = not (before == "" or before.endswith("\n"))
            # A slot inside a line: what ends with a newline continues on a line indented past the slot's line.
            line = before.rsplit("\n", 1)[-1]
            cont = " " * (len(line) - len(line.lstrip(" ")) + 2)
            for t, kind, text in trivia:
                if t == "nothing" and slot != "B":
                    continue
                body = fill(text, k, w)
                if body == "":
                    pass
                elif inline:
                    body = body + cont if body.endswith("\n") else body
                elif body.endswith("\n"):
                    body = indent(body, pad)
                else:
                    body = pad + body
                src = program.replace(marker, body).replace("<B>", "").replace("<I>", "")
                out.append((f"{name}--{t}--{'before' if slot == 'B' else 'between'}", src))
            if "header" in kinds:
                for h, text in HEADERS:
                    out.append((f"{name}--{h}--before", program.replace("<B>", text).replace("<I>", "")))
    made = dict(out)
    for name, variation, layouts in LAYOUTS:
        trivia, slot = variation.split("--")
        for layout in layouts:
            out.append((f"{name}--{layout}--before", laid_out(made[f"{name}--nothing--before"], layout)))
            out.append((f"{name}--{layout}-{trivia}--{slot}", laid_out(made[f"{name}--{variation}"], layout)))
    return out


def generate(d):
    os.makedirs(d, exist_ok=True)
    for f in os.listdir(d):
        if f.endswith(".scala"):
            os.remove(os.path.join(d, f))
    for name, src in cells():
        with open(os.path.join(d, name + ".scala"), "w", encoding="utf-8") as f:
            f.write(src)
    print(f"places: {len(cells())} cells in {d}")


PLACE = re.compile(r"\[s\d+ (\d+)\.\.(\d+)@(\d+) ")


def against_plain(detail, placeless):
    """Prints each line of a judged cell that the head does not have as scalac's: a line of scalac's whose aligned line
    has another place, a line of either dump with no line aligned to it. Each is tagged against its construct's plain
    cell, by the line's index where scalac's two trees are one kind for kind (a dump's own lines by their kind):
    `plain too` (the plain cell's place as far from scalac's), `plain too, moved otherwise`, `variation only`, or why
    there is no such comparison. Returns per cell its lines that are not `plain too`."""
    import collections

    def own(heads, cell):
        """Lines' heads with the cell's own name, which its file's wrapper is named by, written alike in every cell."""
        return [(depth, kind, defined.replace(cell, "<cell>")) for depth, kind, defined in heads]

    def moved(cell, i):
        """How far the head's place of scalac's line `i` is from scalac's: its start's, end's and point's distance."""
        at = [PLACE.search(x) for x in (cell["pos"][i], cell["on_head"][i])]
        if not all(at):
            return cell["on_head"][i]
        return tuple(int(h) - int(o) for o, h in zip(at[0].groups(), at[1].groups()))

    out = {}
    for name, _ in cells():
        if name not in detail:
            continue
        d = detail[name]
        plain = name.split("--")[0] + "--nothing--before"
        p = detail.get(plain)
        if name == plain:
            why = "plain"
        elif p is None:
            why = "no plain cell judged"
        elif own(p["ho"], plain) != own(d["ho"], name):
            why = "scalac's tree is not the plain cell's"
        else:
            why = None
        wrong, alone_o, alone_h = d["side"]
        if why is None:
            pw, po = set(p["side"][0]), set(p["side"][1])
            left = collections.Counter(own(p["hh"], plain)[j] for j in p["side"][2])
            keys = own(d["hh"], name)
        lines = []
        for i in wrong:
            t = why or ("variation only" if i not in pw else "plain too" if moved(d, i) == moved(p, i) else "plain too, moved otherwise")
            base = d["on_base"].get(i)
            also = "" if base == d["on_head"][i] else f" base `{base.strip() if base else 'none'}`"
            lines.append((t, f"NOT SCALAC'S ({t}): scalac `{d['pos'][i].strip()}` head `{d['on_head'][i].strip()}`{also}"))
        for i in alone_o:
            t = why or ("plain too" if i in po else "variation only")
            lines.append((t, f"SCALAC'S ALONE ({t}): `{placeless(d['o'][i].strip())}`"))
        for j in alone_h:
            t = why
            if t is None:
                t = "plain too" if left[keys[j]] > 0 else "variation only"
                left[keys[j]] -= 1
            lines.append((t, f"HEAD'S ALONE ({t}): `{placeless(d['head'][j].strip())}`"))
        for t, text in lines:
            print(f"{name} {text}")
        out[name] = [(t, text) for t, text in lines if t != "plain too"]
    return out


def by_read(verdicts, detail, defeats):
    """Per read of READS: its constructs' cells and their verdicts; each plain cell's lines that are not scalac's (a
    construct written in a varied form, a backquoted name, is its own plain cell) and of its lines of the kinds the read
    places those that are scalac's; each line not scalac's of a variation that its plain cell has not the same way."""
    import collections
    names = [n for n, _ in cells()]
    for n, (fn, kinds) in sorted(READS.items()):
        constructs = [x[0] for x in CONSTRUCTS if n in reads_of(x[0])]
        mine = [m for m in names if m.split("--")[0] in constructs]
        counts = collections.Counter(verdicts[m] for m in mine)
        plains = [x + "--nothing--before" for x in constructs]
        clean = [p for p in plains if p in detail and not defeats.get(p)]
        varied = [m for m in mine if m not in plains and defeats.get(m)]
        print(f"read {n} ({fn}): {len(constructs)} constructs, {len(mine)} cells, {counts['judged']} judged; "
              + "".join(f"{v}: {k}; " for v, k in sorted(counts.items()) if v != "judged")
              + f"plain cells with every line scalac's: {len(clean)} of {len(plains)}; "
              f"variations with a line not scalac's that the plain cell has not the same way: {len(varied)}")
        for p in plains:
            if p not in detail:
                print(f"  plain {p}: {verdicts.get(p, 'no such cell')}")
                continue
            d = detail[p]
            of_kind = [i for i in range(len(d["o"])) if d["ho"][i][1] in kinds.split()]
            right = [i for i in of_kind if d["on_head"].get(i) == d["pos"][i]]
            print(f"  plain {p}: {len(defeats.get(p, []))} lines not scalac's; of its {kinds} lines {len(right)} of {len(of_kind)} scalac's")
            for _, text in defeats.get(p, []):
                print(f"  {p} {text}")
        for m in varied:
            for _, text in defeats[m]:
                print(f"  {m} {text}")


def judge(work, head_dir):
    import compare as c
    import records as rec
    import regressions as r

    def read(p):
        return [x for x in p if not x.startswith("class ")]

    names = list(dict.fromkeys(n for n, _ in cells()))
    # The head's records against the manifest: a cell held twice, a cell with no record, a record of no cell.
    problems = rec.manifest([n for n, _ in cells()], sorted(os.listdir(head_dir)) if os.path.isdir(head_dir) else [])
    rows, lost_total, verdicts, detail = [], 0, {}, {}
    for name in names:
        u = os.path.join(work, name)
        hd = os.path.join(head_dir, name)
        unit = rec.corpus_unit(u)
        (s_state, s_val), (b_state, b_val) = unit["scalac"], unit["base"]
        if s_state == "not judged":
            verdict = "NOT JUDGED: " + s_val
        elif s_state == "refused":
            verdict = "scalac refuses"
        elif b_state == "not judged":
            verdict = "NOT JUDGED: " + b_val
        elif b_state == "refused":
            verdict = "the base makes no products"
        elif not os.path.isdir(hd):
            verdict = "NOT JUDGED: no record of the head"
        elif os.path.exists(os.path.join(hd, "not-judged")):
            verdict = "NOT JUDGED: " + open(os.path.join(hd, "not-judged"), encoding="utf-8").read().strip()
        elif (h := rec.step(os.path.join(hd, "view.dump"), os.path.join(hd, "view.exit"), "the head's view",
                            lambda p: rec.dump(p, side="head")))[0] != "ok":
            verdict = "NOT JUDGED: " + h[1]
        else:
            o, base, head = read(s_val), read(b_val), read(h[1])
            on_base, pos = r.aligned(o, base)
            on_head, _ = r.aligned(o, head)
            both = [i for i in range(len(pos)) if i in on_base and i in on_head]
            # A line scalac's on the base is lost where the head has it elsewhere or has no line aligned to it.
            lost = [i for i in range(len(pos)) if i in on_base and on_base[i] == pos[i] and on_head.get(i) != pos[i]]
            gained = [i for i in both if on_base[i] != pos[i] and on_head[i] == pos[i]]
            neither = [i for i in both if on_base[i] != pos[i] and on_head[i] != pos[i]]
            lost_total += len(lost)
            verdict = "judged"
            rows.append((name, lost, gained, neither, pos, on_base, on_head))
            # The head's dump against scalac's: the lines whose aligned line has another place, and the lines of either
            # that no line of the other is aligned to (another tree where scalac's has its own).
            ho, hh = [c.head(x) for x in o], [c.head(y) for y in head]
            mh = r.matches(ho, hh)
            wrong = sorted(i for i in on_head if on_head[i] != pos[i])
            alone_o = [i for i in range(len(o)) if i not in {i for i, _ in mh}]
            alone_h = [j for j in range(len(head)) if j not in {j for _, j in mh}]
            detail[name] = {"o": o, "ho": ho, "pos": pos, "on_base": on_base, "on_head": on_head, "head": head, "hh": hh,
                            "side": (wrong, alone_o, alone_h)}
        verdicts[name] = verdict
    # Of the lines neither has as scalac's, the head's place is master's (unchanged) or one the head moved: each moved
    # one is a named row; the unchanged ones are master's places, counted by construct and tree kind.
    def split(neither, on_base, on_head):
        moved = [i for i in neither if on_head[i] != on_base[i]]
        return moved, [i for i in neither if on_head[i] == on_base[i]]

    print("| cell | verdict | lost | gained | moved, not scalac's | master's, unchanged |")
    print("|---|---|---|---|---|---|")
    by = {n: (l, g, nb, ob, oh) for n, l, g, nb, _, ob, oh in rows}
    for name, _ in cells():
        if name in by:
            l, g, nb, ob, oh = by[name]
            moved, same = split(nb, ob, oh)
            print(f"| {name} | judged | {len(l)} | {len(g)} | {len(moved)} | {len(same)} |")
        else:
            print(f"| {name} | {verdicts[name]} | | | | |")
    totals = {"lost": 0, "gained": 0, "moved": 0, "same": 0}
    kinds = {}
    for name, lost, gained, neither, pos, on_base, on_head in rows:
        moved, same = split(neither, on_base, on_head)
        totals["lost"] += len(lost)
        totals["gained"] += len(gained)
        totals["moved"] += len(moved)
        totals["same"] += len(same)
        for title, lines in (("LOST", lost), ("MOVED, not scalac's (a row)", moved)):
            for i in lines:
                at = on_head.get(i)
                print(f"{name} {title}: scalac `{pos[i].strip()}` base `{on_base[i].strip()}` head `{at.strip() if at else 'none aligned'}`")
        for i in same:
            key = (name.split("--")[0], pos[i].strip().split(" ", 1)[0])
            kinds[key] = kinds.get(key, 0) + 1
    print("master's places neither has as scalac's, unchanged by the head, by construct and tree kind:")
    for (construct, kind), n in sorted(kinds.items()):
        print(f"  {construct} {kind}: {n}")
    defeats = against_plain(detail, c.placeless)
    by_read(verdicts, detail, defeats)
    judged = len(rows)
    counts = {}
    for v in verdicts.values():
        counts[v] = counts.get(v, 0) + 1
    for p in problems:
        print(f"MANIFEST: {p}")
    not_judged = sum(n for v, n in counts.items() if v.startswith("NOT JUDGED"))
    print(f"places: {len(verdicts)} cells, {judged} judged, {lost_total} lines scalac's on the base and not on the head; "
          f"{totals['gained']} gained; {totals['moved']} moved and not scalac's; {totals['same']} master's unchanged; "
          + "; ".join(f"{v}: {n}" for v, n in sorted(counts.items()) if v != "judged")
          + f"; {not_judged} not judged; {len(problems)} manifest problems")
    return 1 if lost_total or not_judged or problems else 0


# The self-test's cells: a construct's plain cell and a variation, judged; a cell scalac refuses; a cell the base
# refuses. Their dumps are made up: a unit of one definition, positions alike on every side.
FIXTURE = ["s13-colon--nothing--before", "s13-colon--line-comment--before", "s13-colon-oneline--line-comment--before",
           "s13-lambda-unicode--nothing--before"]
# A compiler's refusal on its record: scalac's diagnostics (InitDump's `scalac.txt`) and teq's (the check's output).
SCALAC_REFUSAL = "-- Error: cell.scala:2:2 \n2 |  extension (y: String) def size2 = y.length\n  |  ^^^^^^^^^\n1 error found\n"
TEQ_REFUSAL = "cell.scala:2:27: error: new Array takes one type argument\n    val array = new Array(1)\n1 error found\n"
TASTY_REFUSAL = "cell.scala: cannot write TASTy for a type that did not check in N1\n1 definition cannot be written as TASTy\n"
JVM_REFUSAL = "jvm backend: cell$package$.run: the local o is not in scope of the generated method\n1 construct not supported by the JVM backend\n"
DUMP = ("unit cell.scala\nsource s1 name=cell.scala path=cell.scala content=yes sha256=0\n"
        "  PackageDef [s1 0..30@0 1:0-2:10] pid=<empty>\n"
        "    DefDef [s1 0..30@4 1:0-2:10] sym=<empty>.cell$package$.f\n"
        "      Apply [s1 10..30@12 1:10-2:10] fn=g\n"
        "        Ident [s1 10..11@10 1:10-1:11] sym=<empty>.cell$package$.g\n")


def self_test():
    """The judge on planted records (`--self-test`): a good run passes; each record missing, empty or cut on scalac's,
    the base's or the head's side, a step past its bound or failed, a wrong position or tree, a dump holding its unit
    twice, a manifest holding a cell twice, a head record of a cell the manifest does not hold, and a manifest cell
    with no record fail, the cell named; so does a refusal's exit other than one named by its line on its record (137,
    2, 124, 139, none, 1 with no error line, 3 without the JVM backend's line, scalac's 1 with teq's summary), and a
    view's exit other than 0. teq's 1 with the TASTy writer's summary and its 3 with the JVM backend's are refusals. Each record changed after its
    producer sealed it (its last line gone with its
    line break and exit kept, a line gone from its middle, a byte changed, its sum gone, another cell's record copied
    in) and the base's view copied over the head's is NOT JUDGED for what its sum says; scalac's dump with its last line
    gone and the head placing that line otherwise (the head's dump sealed as its producer wrote it) fails."""
    import records as rec
    t = rec.SelfTest("places.py")
    global cells
    real = cells

    def good(root):
        work, head = os.path.join(root, "work"), os.path.join(root, "head")
        judged, scalac_refuses, base_refuses = FIXTURE[:2], FIXTURE[2], FIXTURE[3]
        for c in FIXTURE:
            u, h = os.path.join(work, c), os.path.join(head, c)
            rec.write(os.path.join(u, "src", "cell.scala"), "val x = 1\n")
            os.makedirs(h, exist_ok=True)
            if c == scalac_refuses:
                rec.write(os.path.join(u, "oracle", "exit"), "1\n")
                rec.put(os.path.join(u, "oracle", "scalac.txt"), SCALAC_REFUSAL, unit=u)
                rec.write(os.path.join(h, "not-judged"), "the head makes no products (exit 1)\n")
                continue
            rec.write(os.path.join(u, "oracle", "exit"), "0\n")
            rec.put(os.path.join(u, "scalac-view.dump"), DUMP)
            rec.write(os.path.join(u, "scalac-view.exit"), "0\n")
            if c == base_refuses:
                rec.write(os.path.join(u, "check.exit"), "1\n")
                rec.put(os.path.join(u, "check.txt"), TEQ_REFUSAL)
                rec.write(os.path.join(h, "not-judged"), "the head makes no products (exit 1)\n")
                continue
            rec.write(os.path.join(u, "check.exit"), "0\n")
            rec.put(os.path.join(u, "view.dump"), DUMP)
            rec.write(os.path.join(u, "view.exit"), "0\n")
            rec.put(os.path.join(h, "view.dump"), DUMP, side="head")
            rec.write(os.path.join(h, "view.exit"), "0\n")
        return work, head

    def run(name, mutate, names=None, want_rc=1, want=None):
        root = t.dir(name)
        work, head = good(root)
        cell = FIXTURE[0]
        mutate(os.path.join(work, cell), os.path.join(head, cell), work, head)
        globals()["cells"] = lambda: [(n, "") for n in (names or FIXTURE)]
        try:
            rc, out = t.quiet(judge, work, head)
        finally:
            globals()["cells"] = real
        rows = {r.split("|")[1].strip(): r.split("|")[2].strip() for r in out.splitlines() if r.startswith("| ") and r.count("|") >= 3}
        ok = rc == want_rc and (want is None or want(rows, out))
        t.expect(name, ok, f"exit {rc}; rows {rows}")

    def nothing(*_):
        pass

    def at(path, u, h):
        return os.path.join({"u": u, "h": h}[path[0]], path[1])

    def side(path):
        return "head" if path[0] == "h" else None

    def drop(path):
        return lambda u, h, w, hd: os.remove(at(path, u, h))

    def blank(path):
        # An empty output, as its producer wrote it and sealed it.
        return lambda u, h, w, hd: rec.put(at(path, u, h), "", side=side(path))

    def cut(path):
        def f(u, h, w, hd):
            p = at(path, u, h)
            data = open(p, encoding="utf-8").read()
            rec.write(p, data[: len(data) // 2])
        return f

    def put(path, content):
        # A record as its producer wrote it: an output sealed, a marker or an exit as it is.
        def f(u, h, w, hd):
            p = at(path, u, h)
            if p.endswith(".dump"):
                rec.put(p, content, side=side(path))
            else:
                rec.write(p, content)
        return f

    named = lambda rows, out: rows.get(FIXTURE[0], "").startswith("NOT JUDGED")
    by_sum = lambda rows, out: named(rows, out) and any(w in rows.get(FIXTURE[0], "") for w in rec.SUM_WHY)
    run("a good run", nothing, want_rc=0, want=lambda rows, out: rows.get(FIXTURE[0]) == "judged" and rows.get(FIXTURE[1]) == "judged"
        and rows.get(FIXTURE[2]) == "scalac refuses" and rows.get(FIXTURE[3]) == "the base makes no products")
    records = (("scalac's view", ("u", "scalac-view.dump")), ("the base's view", ("u", "view.dump")), ("the head's view", ("h", "view.dump")))
    for what, path in records:
        run(f"{what} missing", drop(path), want=named)
        run(f"{what} empty", blank(path), want=named)
        run(f"{what} cut", cut(path), want=named)
    for what, path in (("scalac's exit", ("u", "oracle/exit")), ("the view of scalac's pickles' exit", ("u", "scalac-view.exit")),
                       ("the base's view's exit", ("u", "view.exit"))):
        run(f"{what} missing", drop(path), want=named)
        run(f"{what} 124", put(path, "124\n"), want=named)
    run("the base's check past its bound", lambda u, h, w, hd: (os.remove(os.path.join(u, "view.exit")), rec.write(os.path.join(u, "check.exit"), "124\n")), want=named)
    run("the head's step past its bound (its NOT JUDGED marker)", put(("h", "not-judged"), "the head's view past its bound\n"), want=named)
    run("a wrong position on the head", put(("h", "view.dump"), DUMP.replace("Ident [s1 10..11@10", "Ident [s1 10..11@11")),
        want=lambda rows, out: " LOST: " in out)
    run("a wrong tree on the head", put(("h", "view.dump"), DUMP.replace("        Ident [s1 10..11@10 1:10-1:11] sym=<empty>.cell$package$.g\n",
                                                                         "        Select [s1 10..11@10 1:10-1:11] sym=<empty>.cell$package$.g\n")),
        want=lambda rows, out: " LOST: " in out)
    run("the head's trees cut at a line", put(("h", "view.dump"), "".join(DUMP.splitlines(keepends=True)[:-1])), want=lambda rows, out: " LOST: " in out)
    run("a dump holding its unit twice", put(("h", "view.dump"), DUMP + DUMP), want=named)
    run("a manifest holding a cell twice", nothing, names=FIXTURE + [FIXTURE[1]])
    run("a head record of a cell the manifest does not hold", lambda u, h, w, hd: os.makedirs(os.path.join(hd, "s13-stray--nothing--before")))
    run("a manifest cell with no record of the head", lambda u, h, w, hd: shutil.rmtree(h), want=named)
    run("a manifest cell with no record of the corpus run", lambda u, h, w, hd: shutil.rmtree(u), want=named)
    # Each record changed after its producer sealed it, the same record of the next cell (sealed by its producer, its
    # dump the same) the other cell's.
    other = lambda p: p.replace(FIXTURE[0], FIXTURE[1])
    for what, path in records + (("the head's view's exit", ("h", "view.exit")),):
        if path[1].endswith(".exit"):
            run(f"{what} missing", drop(path), want=named)
            run(f"{what} 124", put(path, "124\n"), want=named)
            continue
        for plant, mutation in rec.after_the_producer(other):
            run(f"{what}: {plant}", lambda u, h, w, hd, path=path, mutation=mutation: mutation(at(path, u, h)), want=by_sum)
    run("the base's view copied over the head's (its output and sum)",
        lambda u, h, w, hd: rec.copy_record(os.path.join(u, "view.dump"), os.path.join(h, "view.dump")), want=by_sum)

    def reviewers_cut(u, h, w, hd):
        # The head places the last Ident otherwise (sealed as its producer wrote it); scalac's dump loses that line
        # after its producer sealed it, its line break and its exit kept.
        rec.put(os.path.join(h, "view.dump"), DUMP.replace("Ident [s1 10..11@10", "Ident [s1 10..11@11"), side="head")
        rec.cut_last_line(os.path.join(u, "scalac-view.dump"))
    run("scalac's last line gone after its producer, the head placing it otherwise", reviewers_cut, want=by_sum)
    # A step's exit by the whitelist: 0, or 1 with the compiler's error line on its record. A signal's (137 a kill, 139
    # a segmentation fault), 2, 124, none recorded and 1 with no error line are NOT JUDGED: on the cell scalac refuses
    # (its InitDump's exit), on the cell the base refuses (its check's), and on the judged cell's views, the head's too.
    def exit_plant(cell, path, value, log=None):
        def f(u, h, w, hd):
            root = {"u": os.path.join(w, cell), "h": os.path.join(hd, cell)}[path[0]]
            p = os.path.join(root, path[1])
            if value is None:
                os.remove(p)
            else:
                rec.write(p, value)
            if log:
                rec.put(os.path.join(root, log[0]), log[1], unit=root)
        return f
    named_cell = lambda cell: (lambda rows, out: rows.get(cell, "").startswith("NOT JUDGED"))
    bad_exits = (("137 (killed)", "137\n"), ("2", "2\n"), ("124", "124\n"), ("139", "139\n"), ("missing", None))
    for cell, path, log, quiet in ((FIXTURE[2], ("u", "oracle/exit"), "oracle/scalac.txt", "warning: none\nanother line\n"),
                                   (FIXTURE[3], ("u", "check.exit"), "check.txt", "compiling\nanother line\n")):
        for what, value in bad_exits:
            run(f"{path[1]} of the refused cell {what}", exit_plant(cell, path, value), want=named_cell(cell))
        run(f"{path[1]} of the refused cell 1 with no error line", exit_plant(cell, path, "1\n", (log, quiet)), want=named_cell(cell))
        run(f"{path[1]} of the refused cell 3 without the JVM backend's line", exit_plant(cell, path, "3\n", (log, quiet)), want=named_cell(cell))
    # teq's two refusals closed by a summary line (src/main.rs): 1 with the TASTy writer's, 3 with the JVM backend's,
    # each a refusal of the base; scalac's exit 1 with such a line, none of its own, is not.
    for what, code, log in (("1 with the TASTy writer's summary", "1\n", TASTY_REFUSAL), ("3 with the JVM backend's summary", "3\n", JVM_REFUSAL)):
        run(f"check.exit of the refused cell {what}", exit_plant(FIXTURE[3], ("u", "check.exit"), code, ("check.txt", log)), want_rc=0,
            want=lambda rows, out: rows.get(FIXTURE[3]) == "the base makes no products")
    run("oracle/exit of the refused cell 1 with teq's summary line", exit_plant(FIXTURE[2], ("u", "oracle/exit"), "1\n", ("oracle/scalac.txt", TASTY_REFUSAL)),
        want=named_cell(FIXTURE[2]))
    # teq's internal error is no refusal, whatever summary follows it.
    run("check.exit of the refused cell 1 with an internal error before the TASTy writer's summary",
        exit_plant(FIXTURE[3], ("u", "check.exit"), "1\n", ("check.txt", "internal error: cell.scala: parsed without the record of the places the TASTy writer reads\n" + TASTY_REFUSAL)),
        want=named_cell(FIXTURE[3]))
    for path in (("u", "oracle/exit"), ("u", "scalac-view.exit"), ("u", "view.exit"), ("h", "view.exit")):
        for what, value in bad_exits[:2] + bad_exits[3:4]:
            run(f"{'the head' if path[0] == 'h' else 'the corpus'}'s {path[1]} {what}", exit_plant(FIXTURE[0], path, value), want=named)
    run("the head placing scalac's last line otherwise, scalac's dump whole",
        put(("h", "view.dump"), DUMP.replace("Ident [s1 10..11@10", "Ident [s1 10..11@11")),
        want=lambda rows, out: " LOST: " in out and "1 lines scalac's on the base and not on the head" in out)
    return t.finish()


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        import shutil
        sys.exit(self_test())
    if len(sys.argv) == 3 and sys.argv[1] == "generate":
        generate(sys.argv[2])
    elif len(sys.argv) == 4 and sys.argv[1] == "judge":
        sys.exit(judge(sys.argv[2], sys.argv[3]))
    else:
        sys.exit(__doc__)
