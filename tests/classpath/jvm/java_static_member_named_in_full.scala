// jars: scala-library
// std: scala-library
// targets: jvm
// The static member class named in full inside the subclass, beside a `Double` that is `scala`'s.
class Mine(a: Int) extends java.awt.Point(a, a):
  def far: java.awt.geom.Point2D.Double = new java.awt.geom.Point2D.Double(a, 2)
  def near: Double = far.getX

@main def run(): Unit =
  println(new Mine(3).near)
