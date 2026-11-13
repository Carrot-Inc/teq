// jars: scala-library
// std: scala-library
// targets: jvm
// A Java class's static member class is no member of a self type's `this`: `Double` inside a trait whose
// self type is `java.awt.geom.Point2D` is `scala.Double`.
trait Marks { self: java.awt.geom.Point2D =>
  def half: Double = getX / 2
}
class Spot extends java.awt.Point(3, 4) with Marks

@main def run(): Unit =
  println(new Spot().half)
