// jars: scala-library scala2-lib
// std: scala-library
// The companion of a case class compiled by Scala 2.13 extends `AbstractFunction1`, so it is the
// function from its field to the class, as scalac 3 reads the pickle; its `apply` stays the
// constructor.
import scala2lib.*

object Main:
  def main(args: Array[String]): Unit =
    val f: Double => Square = Square
    println(f(2.0))
    println(List(1.0, 3.0).map(Square))
    println(Square(5.0).side)
    println(Square.andThen(_.side)(7.0))
