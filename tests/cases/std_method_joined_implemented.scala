// A class that joins two of the standard library's function types changes nothing for a class of the program that
// implements one of them: an anonymous partial function collects and lifts, an anonymous function class maps, on
// every target, as under scalac 3.8.4.
class Twice extends (Int => Int) with ((Int, Int) => Int) {
  def apply(x: Int): Int = x + 10
  def apply(x: Int, y: Int): Int = x + y
}

object Main {
  def main(args: Array[String]): Unit = {
    val own = new PartialFunction[Int, Int] { def isDefinedAt(x: Int): Boolean = x > 0; def apply(x: Int): Int = x * 2 }
    println(List(-1, 2).collect(own))
    println(own.lift(3))
    val inc = new Function1[Int, Int] { def apply(x: Int): Int = x + 1 }
    println(List(1, 2).map(inc))
  }
}
