// `Desugar.makeFor`'s generator modes (`GenCheckMode`): a `case` generator's `FilterAlways` filters its pattern before every
// selection, again after each guard under better fors (3.8) and never again before (`Ignore`); a guard's `Filtered` keeps
// the trailing `map`, which `FilterAlways` leaves out of an identity; the pattern filter and a `case` lambda match an
// identifier as a stable identifier pattern; an alias's given in parentheses of its own is nested (`isNestedGivenPattern`).
// teq: --source 3.3
class Box[A](val x: A) {
  def map[B](f: A => B): Box[B] = { println("map"); new Box(f(x)) }
  def flatMap[B](f: A => Box[B]): Box[B] = { println("flatMap"); f(x) }
  def withFilter(f: A => Boolean): Box[A] = { println(s"filter ${f(x)}"); this }
}
object P {
  val K = 1
  def main(args: Array[String]): Unit = {
    println("case guard"); println((for { case x <- new Box(1); if x > 0 } yield x + 1).x)
    println("case two guards"); println((for { case x <- new Box(1); if x > 0; if x < 5 } yield x + 1).x)
    println("case identity"); println((for { case x <- new Box(1) } yield x).x)
    println("case guard identity"); println((for { case x <- new Box(1); if x > 0 } yield x).x)
    println("guard identity"); println((for { x <- new Box(1); if x > 0 } yield x).x)
    println("case stable id"); println(for { case K <- List(1, 2) } yield "k")
    println("paren identity"); println((for { x <- new Box(1) } yield (x)).x)
    println("paren given"); println((for { x <- new Box(1); (given Int) = x } yield summon[Int] + 1).x)
    println("case guard alias generator"); println((for { case x <- new Box(1); if x > 0; y = x; z <- new Box(y) } yield z).x)
    println("case alias guard"); println((for { case x <- new Box(1); y = x; if y > 0 } yield y + 1).x)
  }
}
