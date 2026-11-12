// A term member selected through a type prefix is a `TermRef` over that prefix (`A#v.type`), kept until a reader widens
// it (`NamedType.withPrefix`, Types.scala 2904-2934). The function's result `c.v.type`, with `c` approximated away
// (`nonDependentResultApprox`, `SubstApproxMap` over `c := Nothing..A`), is `A#v.type`, a singleton of `O.s.type`, so
// `f` conforms to `A => O.s.type`, as scalac 3.8.4 accepts it. teq's `Select` takes a path prefix, so a term member
// through the type `A` is widened to `String`, and `f` is refused (the adversary of the pass on 25e6342fb, X6). Moves
// with type-representation's landing 4, references.
object O:
  val s: String = "s"
class A:
  val v: O.s.type = O.s
def h(f: (c: A) => c.v.type): A => O.s.type = f
@main def run(): Unit =
  val g = h(c => c.v)
  println(g(A()))
