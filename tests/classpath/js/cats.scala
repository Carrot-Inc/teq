// jars: scala-library cats-kernel cats-core
// cats type classes and their syntax against the real jars: the instances (cats.kernel.instances,
// cats.instances), Show, Eq, Semigroup and Monoid, Validated and the Either and Foldable syntax
// are compiled from their TASTy bodies over the std's List, Option, Either and Map.
import cats.*
import cats.data.Validated
import cats.syntax.all.*

case class Point(x: Int, y: Int)
object Point:
  given Eq[Point] = Eq.fromUniversalEquals
  given Show[Point] = Show.show(p => s"Point(${p.x}, ${p.y})")

object Main:
  def main(args: Array[String]): Unit =
    println(1 |+| 2)
    println("a" === "a")
    println("a" =!= "b")
    println(List(1, 2).show)
    println(Eq[Point].eqv(Point(1, 2), Point(1, 2)))
    println(Eq[Point].neqv(Point(1, 2), Point(2, 1)))
    println(Point(3, 4).show)
    println(Option(5).show)
    val v: Validated[String, Int] = Validated.valid(3)
    println(v.map(_ + 1))
    println(Validated.invalid[String, Int]("bad") |+| Validated.invalid("worse"))
    println(v.toEither)
    val e: Either[String, Int] = Right(2)
    println(e.map(_ * 3).toOption)
    println(List(1, 2, 3).foldMap(_ * 2))
    println(List("a", "b").foldMap(_.length))
    println(Semigroup[Int].combine(3, 4))
    println(Monoid[String].empty.isEmpty)
    println("x".combineN(3))
