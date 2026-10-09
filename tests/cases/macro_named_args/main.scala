import Macros.*

case class P(name: String, age: Int, tags: List[String] = Nil)
class Greeter:
  def greet(first: String, second: String): String = s"$first then $second"
  def combine(x: Int, y: Int): Int = x * 10 + y

@main def run(): Unit =
  val p = P("ann", 30)
  println(setField(p, "age", 31))
  println(setField(p, "name", "bob"))
  println(setField(p, "tags", List("x")))
  println(setField(P("c", 1, List("t")), "age", 2))
  val g = new Greeter
  println(callReversed(g, "greet", "a", "b"))
  println(callReversed(g, "combine", 1, 2))
  println(describeNamed("count", 7))
