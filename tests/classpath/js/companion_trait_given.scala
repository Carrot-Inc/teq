// jars: fixtures
// A library body passes a given its companion object inherits from a trait, by the object's path
// (tests/tasty/src/compdsl.scala): the given is read on that object.
import fix.shapes.CompDsl.*

@main def run(): Unit =
  val r = new RouteB[Int]("a") / new RouteB[Unit]("b")
  println(r.name)
