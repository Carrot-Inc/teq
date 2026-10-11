# Scala 3 compile-only tests against teq

`harness.py` runs `tests/neg` and `tests/pos` of a scala3 checkout through `teq compiler check` and
classifies every test (see the docstring for the categories). The sources are not copied: set
`SCALA3` to the checkout (default `../teq-ref/scala3`) and `TEQ` to the
binary (default `target/release/teq` of this repository).

```
python3 tests/scala3-typing/harness.py                 # compare with expected.txt, exit 1 on a change
python3 tests/scala3-typing/harness.py --update        # rewrite expected.txt
python3 tests/scala3-typing/harness.py --list accepts --suite neg   # the neg tests teq accepts
python3 tests/scala3-typing/harness.py --show neg/bounds.scala      # teq's output for one test
```

teq's output is kept up to its first 1 MiB and last 4 KiB (`HEAD_CAP`, `TAIL_CAP`), the rest
read and dropped, so a test that prints millions of errors is classified by its first errors
and its count.

`expected.txt` holds one line per test (`suite/name`, category, detail); only the category is
compared, so the file doubles as a regression check when the compiler changes: a neg test moving
from `accepts` to `same` is progress, a pos test moving from `accepts` to `rejects` is a
regression.

- `probes/` are the minimal programs the differences of the suite's run were reduced to
  (in three groups, `p*`, `q*` and `s*`, and `run/`, programs whose output was compared with
  `scala-cli run`). Run one with `teq compiler check` and with
  `scala-cli compile -S 3.8.4 --server=false`.
- `proposed-errors/` are error tests in the format of `tests/errors` (`// expect:` lines) for
  the rules teq does not implement yet; the expected messages are proposals. They are kept out
  of the live suite because teq accepts most of these programs today.
- `proposed-cases/` are programs scalac accepts and teq rejects, with the output of
  `scala-cli run -S 3.8.4` as `.expected`, in the format of `tests/cases`.

## A given object's path widens to its class (2026-10-10)

A path to a given object (`given T with { .. }`, a module in dotty) widens to the object's class, as a
module's `TermRef` widens to its module class, and a stable candidate found for a target that holds a
path is that reference:

- `pos/i12910` moved from `rejects` to `accepts`: `t.Out` of the given `Type[varchar]` with `type Out =
  String` is `String`, so `Placeholder[varchar]` is a `Placeholder[varchar, String]`.
- `pos/i25417` moved from `rejects` to `accepts`: `mirror.IronType` of the inline given `Mirror[Foo.T]`
  with `type IronType = String` is `String`, so `Decoder[mirror.IronType]` finds `Decoder[String]` and
  the `summonFrom` reduces.

## An object nested in a class is read through its prefix (2026-10-10)

A member a class or trait declares, read on an object or a path, keeps that receiver in its path
(dotty's `TermRef(prefix, member)`), and an object's class conforms to the object's path through a
conforming prefix (`TypeComparer.compareNamed`, `X$ <:< X.type`):

- `neg/moduleSubtyping` moved from `subset` to `same`: `Test.o.type` is no `C.this.o.type`, so the
  four assignments between them are refused, where `Test.o` read as the bare `o` let three through.
- `pos/i26656` moved from `rejects` to `accepts`: the transparent `Parser` gives `this$0.Parser1`, the
  object's class through the receiver's proxy, which conforms to the extension's `Parser1.type`
  through `ImmutLexerFlow`.
- `pos/i14783` and `pos/i8623` stay `rejects` with another first error: the extension of an object
  nested in a trait is found through its path (`w.x.foo` over `ctx.inner.X`, `unseal.pos` over
  `qctx.tasty.Tree`); what remains is `quotes.reflect.Tree` through two paths in `i14783` and
  `given QC()` of a trait in `i8623`.

## An intersection's members met as dotty's `findMember` and `mergeSingleDenot` (2026-10-10)

`neg/i7425` moved from `superset` to `same`: `C & D` with `C.f: Int` and `D.f(): Int` keeps the two
members apart (dotty's `MultiDenotation`, a value-like method against one of an empty clause), and the
set that stands for them is a view no class owns, so the naming pass no longer takes `C.f` and `D.f` for
alternatives of `C` and reports their JavaScript names (`f$0`); the lines left are scalac's own.

`pos/i9531` moved from `rejects` to `accepts`: a literal argument is of its literal type to overload
resolution (dotty's `ConstantType`), so `exprQuote[T <: Singleton](x: T)` applies to `1` beside
`exprQuote[T](x: T)(using s, dummy: Null = null)`, is the more specific, and infers `T` as `1`.

`neg/i8736` stays `superset`, for the lines its `{}` type adds; the three calls' diagnostics are
asserted as scalac's (two `Any` results refused, one ambiguity among the refinements) by
`tests/errors/intersection_member_i8736.scala`.

## A member's retry types again what its application did not cache (2026-10-09)

`pos/i18645` moved from `rejects` to `accepts`: `x.pprint(() => ( 234 ))` fails against the member's
`() => String` at the lambda's result, which is no error inside the argument (`hasInnerErrors`), and its
typing, made where the member's attempt had reported that error, is not cached (`cacheTypedArg`), so the
retry on the qualifier types the lambda again against the extension's `() => Int`, as scalac's
`tryWithImplicitOnQualifier` does.

## A case class is a `java.io.Serializable` (2026-10-09)

`pos/duplicate-parents` moved from `rejects` to `accepts`: a case class has the parents
`Desugar.classDef` gives it, `Product` and `java.io.Serializable`, so `f: scala.Serializable` of a case
class `Foo5` (which also extends a local class named `Serializable`) conforms, as under scalac.

## Types on the next line (2026-10-02)

Thirty-seven tests moved when a type started on the next, indented line after the `=` of an
alias, the `=>` of a function type or a match type's case, or the `:` of an annotation, and a
generic `unapply` was instantiated against the
scrutinee. Each was a parse error before:

- `syntax` to `accepts` (19): `pos/10867`, `13469`, `13491`, `22219b`, `bad-footprint`,
  `i14903a`, `i19907_slow_1000_3`, `i19907_slow_1000_4`, `i19907_slow_100_3`, `i20897`, `i22944`,
  `i5650`, `i5666`, `listpattern`, `mt-scrutinee-widen3`, `t2082`, `t5643`, `t7785`; and
  `neg/mt-scrutinee-widen2`, whose rule is missing: a match type over `a.type` for a parameter
  `a` does not reduce, where teq widens the scrutinee and reduces both sides to the same case.
- `oos` to `same`: `neg/i6503` (a val's type on the next line, at the val's own indentation).
- To a later gap, the parse error gone: `rejects` (8: `pos/14952`, `6709`, `9239`,
  `conversion-function-prototype`, `i12141`, `i14903b`, `i19445`, `i25947`, typing gaps of match
  types, a refinement's `Selectable` and a function prototype), `superset` and `different`
  (`neg/i15352`, `i23552`, `i8736`, `i19445`, `i4369c`, `mt-scrutinee-widen`), `std`
  (`neg/16463`'s `Take`; `pos/curried-colon-lambda`, whose single-line colon lambda `fun: (x:
  Int) => (y: Int) => x + y` is read as an ascription), and `syntax` from `oos` (`neg/i6501`,
  whose val type is `MapImpl & {type Key = String}`, a refinement as an operand of `&`).

A detail moved within its category: `pos/t4911` keeps one error of two (`Foo.unapply[T]` over an
`M[T]` types). With the member of an intersection taken from the part whose result type is the
narrower, `pos/intersection` moved from `rejects` to `accepts` (`(??? : B & A).f` is `B`'s `Int`).

## A negative test teq accepts since a leading operator follows scalac (2026-09-29)

`transparent-inline-nested-note` moved from `syntax` to `accepts` when a line's leading operator
got scalac's `isLeadingInfixOperator`: `??? : one.Out` is a statement of its own, a `:` being no
operand. The rule scalac rejects the program with is missing: inside the body of a transparent
inline method the result of another one (`internal[1]`) keeps its declared type `Id[1]`, so
`one.Out` is `Int` and `val i: 1 = id` a mismatch.

## Negative tests teq accepts since objects nested in classes type (2026-09-28)

Eight `neg` tests moved from `oos` to `accepts` when nested objects were supported: their
programs type now, and the rule scalac rejects each with is missing. Each is a check of its
own, queued under language completeness:

- `i11100`: an `@static` method must belong to an object that is static; `C.D.foo` belongs to
  an instance's object and reads `C.this.a`.
- `i14660`: `private` and `opaque` cannot be combined on a type alias.
- `i15939`: the companion of a class nested in a class enters the implicit scope through a
  stable prefix only; `mkFoo.andThen(mkBarString)` wants a `mkFoo.Bar` whose prefix is a method
  call, so `Bar.fromString` is out of scope (`proposed-errors/unstable_prefix_companion.scala`).
- `i22560b`, `i22560c`: a constructor proxy (`Val(27)`, `p.C(42)`) carries the class's access:
  `private`, `private[p]` and `protected` (the proxy's `apply` is a member of the companion, so
  a subclass of the enclosing class cannot reach a protected one).
- `i3364`: a class `Foo` nested in `class Test` and an object `Foo` nested in `object Test`
  clash, as their companion module classes would.
- `inner-classes-of-universal-traits`: a value class cannot be a member of another class, and a
  universal trait (`extends Any`) cannot hold an object or a case class.
- `t4815`: `final` is not allowed on a local def, val, var or type.
