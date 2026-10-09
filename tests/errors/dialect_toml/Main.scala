// expect: `count` needs a declared type under the dialect flag `explicit-result-types`
// expect: the call of the inline method `twice` is not allowed under the dialect flag `no-inline`
// The teq.toml next to this file turns the strict dialect on without no-overloading: the
// alternatives of go pass, the inferred type and the inline call are rejected.
object O:
  def go(x: Int): Int = x
  def go(x: String): Int = x.length
  val count = 3
  inline def twice(x: Int): Int = x + x

@main def main(): Unit =
  println(O.go(1) + O.go("a") + O.count + O.twice(1))
