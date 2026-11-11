// A call of a function value that may be an object (a Map is an `Int => Int`) goes through a helper of the
// JavaScript runtime; a local of the body named as that helper keeps its value and the call its meaning.
object Main:
  def f(g: Int => Int): Int =
    val $jsFn = 1
    g(2) + $jsFn

  def pick(): Int => Int = Map(2 -> 3)

  def h(): Int =
    val $ap = 10
    pick()(2) + $ap

  def main(args: Array[String]): Unit =
    println(f(x => x * 10))
    println(f(Map(2 -> 3)))
    println(h())
