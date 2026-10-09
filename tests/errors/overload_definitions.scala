// expect: 12:7: error: render is already defined as method render
// A val next to a method of its name is an alternative, as scalac keeps it.
// expect: 19:7: error: Conflicting definitions:
// expect: def total(xs: List[Int]): Int in object Shop at line 18 and
// expect: def total(xs: List[String]): Int in object Shop at line 19
// expect: have the same type after erasure.
// expect: 21:7: error: price is already defined as method price
// expect: 23:7: error: two or more overloaded variants of method ship have default arguments
// expect: 5 errors found
def show(): Int =
  def render(n: Int): String = "#" + n
  def render(s: String): String = s
  render(1).length

object Shop:
  def id(n: Int): Int = n
  val id: String = "shop"
  def total(xs: List[Int]): Int = xs.sum
  def total(xs: List[String]): Int = xs.length
  def price(item: Int): Int = item
  def price(item: Int): Int = item * 2
  def ship(item: Int, days: Int = 2): String = "a"
  def ship(item: String, days: Int = 3): String = "b"

@main def run(): Unit = println(show())
