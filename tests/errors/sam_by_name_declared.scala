// expect: 6:14: error: type mismatch: found Int => Int, required S
// expect: 1 error found
// A lambda implementing a method with a by-name parameter takes no declared type for it.
trait S { def f(x: => Int): Int }
@main def main(): Unit =
  val s: S = (x: Int) => x
  val t: S = x => x
  println(t.f(1))
