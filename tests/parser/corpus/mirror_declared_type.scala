// A product mirror summoned in a body: under --std=scala-library (the stdlib suite) its one value
// is declared with the mirror's refined type, which the body's worker makes and the loader's lock
// holder publishes as a shared symbol's signature, in the base's types.
import scala.deriving.Mirror

case class Point(x: Int, y: Int)

@main def main(): Unit =
  val m = summon[Mirror.Of[Point]]
  println(m.fromProduct((3, 4)))
  val p = summon[Mirror.ProductOf[Point]]
  println(p.fromProduct((1, 2)).x)
