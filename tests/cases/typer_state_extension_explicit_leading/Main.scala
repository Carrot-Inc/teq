// A lexical extension whose leading using clause the call passes itself (`f(using new X(4))`),
// beside a companion's: the probe of its using clauses resolves none the call passes, so the
// inline given that would fail is not expanded (scalac `41`).
class X(val v: Int)
object X:
  inline given x: X = scala.compiletime.error("UNUSED GIVEN")

class C
object C:
  extension (c: C) def f(using x: X)(k: Int): Int = x.v * 10 + k

extension (c: C)(using x: X) def f(k: Int): Int = x.v * 10 + k

@main def run(): Unit = println(C().f(using new X(4))(1))
