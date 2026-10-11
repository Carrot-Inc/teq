// Calls with defaults whose plan the writer writes as scalac's typer makes it
// (`Applications.TypedApply`), which the pinned census counts as written
// (tests/tasty/expected/bodies.census): an earlier by-name argument, an intervening using clause,
// a repeated earlier clause, and a later clause's named arguments out of order after an earlier
// clause's default.
package probe.defaults

object Withheld {
  def mark(s: String): Int = { print(s); 0 }
  def f1(a: => Int)(b: Int = a): Int = a + b
  def f3(a: Int = mark("A"))(using u: Int)(b: Int = mark("B")): Unit = ()
  def f4(xs: Int*)(b: Int = mark("B")): Unit = ()
  class K(a: Int = mark("A"), b: Int)(c: Int, d: Int)
  def byName(): Unit = println(f1(mark("T"))())
  def using(): Unit = { f3()(using mark("U"))(); println() }
  def repeated(): Unit = { f4(mark("A"))(); println() }
  def hoisted(): Unit = { new K(b = 0)(d = mark("D"), c = mark("C")); println() }
  def main(args: Array[String]): Unit = { byName(); using(); repeated(); hoisted() }
}
