// Overloading on a literal argument sees its constant type (`resolveOverloaded` over the argument
// typed alone): `pick(1)` takes the alternative over `1`, `pick(2)` the one over `Int`, and a
// numeric formal still takes the widened constant.
import scala.annotation.targetName
@targetName("pickOne") def pick(x: 1): String = "one"
def pick(x: Int): String = "int"
def f(x: Double): String = "double"
def f(x: String): String = "string"
@targetName("tagTrue") def tag(b: true): String = "yes"
def tag(b: Boolean): String = "bool"
@targetName("nameA") def name(s: "a"): String = "A"
def name(s: String): String = "string"
@main def run() =
  println(pick(1))
  println(pick(2))
  println(f(1))
  println(tag(true))
  println(tag(false))
  println(name("a"))
  println(name("b"))
