// An explicit leading `using` argument leaves no given the probe resolved queued for its
// expansion: the failing inline given is never expanded (41), as scalac and master print.
class X(val v: Int)
object X:
  inline given x: X = scala.compiletime.error("UNUSED GIVEN")
class C
object C:
  extension (c: C) def f(using x: X)(k: Int): Int =
    x.v * 10 + k
extension (c: C)(using x: X) def f(k: Int): Int =
  x.v * 10 + k
@main def run(): Unit = println(C().f(using new X(4))(1))
