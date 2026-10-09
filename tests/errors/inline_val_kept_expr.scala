// expect: 12:11: error: inline value must have a literal constant type
// expect: inlined from tests/errors/inline_val_kept_expr.scala:8
// expect: 1 error found
// An `inline val` over a call the definition keeps is checked where the body expands: one that
// is no constant there is scalac 3.8.4's error at the call, with its inline stack trace.
inline def f1(): Long = 1L
inline def g(y: Long): Long =
  inline val x = f1() + y
  x

@main def run(): Unit =
  println(g(System.nanoTime() % 2))
