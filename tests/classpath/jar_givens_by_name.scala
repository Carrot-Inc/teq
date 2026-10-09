// jars: fixtures scala-library
// Givens of a jar imported by name, outside the implicit scope of their types: a package-level
// given of a package object and a given alias.
import fix.shapes.{topGiven, Shape, Rect}
import fix.shadow.{defaultShadowed, Shadowed}

object Main:
  def main(args: Array[String]): Unit =
    val o: Ordering[Shape] = summon[Ordering[Shape]]
    val s: Shadowed = summon[Shadowed]
    println(o.compare(Rect(1.0), Rect(2.0)) + s.which)
