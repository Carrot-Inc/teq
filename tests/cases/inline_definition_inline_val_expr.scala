// An `inline val` whose value is an expression over a call the definition keeps for the
// expansion (`f1() + 1L`) is checked where the body expands, as scalac 3.8.4 checks it, and
// stands for its constant there. One whose value is the call alone (`inline val x = f1()`) is
// "inline value must have a literal constant type" at the expansion under 3.8.4
// (tests/errors/inline_val_ascribed_body.scala), where scala3's later test accepts it. From
// scala3 tests/run/i24420-inline-val.scala (Apache-2.0, see tests/scala3/README.md).
inline def f1(): Long = 1L
inline def f2(): Long =
  inline val x = f1() + 1L
  x
inline def g1(): Boolean = true
inline def g2(): Long = inline if g1() then 1L else 2L
inline def g3(): Long = inline if f1() > 0L then 1L else 2L

@main def run(): Unit =
  println(f2())
  println(g2() + g3())
