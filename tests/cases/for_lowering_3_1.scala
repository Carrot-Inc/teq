import scala.language.`3.1`
// The for comprehension's lowering at 3.1 (dotty's `GenCheckMode` by `Parsers.generatorRest`, `Desugar.makeFor`): a
// generator's pattern filtered by its syntax before 3.4, aliases packed by a `map` of their own and no trailing `map`
// left out before 3.8; a refutable pattern filtered before 3.4.
// teq: --wconf msg=more.specialized:s
class Box[A](val a: A) {
  def map[B](f: A => B): Box[B] = { println("map"); new Box(f(a)) }
  def flatMap[B](f: A => Box[B]): Box[B] = { println("flatMap"); f(a) }
  def withFilter(f: A => Boolean): Box[A] = { println("filter"); if !f(a) then throw new Exception("filtered"); this }
}

object P {
  def main(args: Array[String]): Unit = {
    val xs = new Box((1, 2))
    println(s"tuple ${(for ((a, b) <- xs) yield a + b).a}")
    println(s"identity ${(for (x <- new Box(7)) yield x).a}")
    println(s"aliases ${(for (x <- new Box(1); y = x + 1; z <- new Box(y)) yield z).a}")
    println(s"trailing ${(for (x <- new Box(1); y = x * 3) yield y).a}")
    println(s"guard ${(for (x <- new Box(3) if x > 0) yield x).a}")
    println(s"case ${(for (case (a, b) <- xs) yield a).a}")
    println(for (Some(x) <- List(Option(1), None)) yield x)
  }
}
