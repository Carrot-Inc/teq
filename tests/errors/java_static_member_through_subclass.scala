// jars: scala-library
// teq: --target jvm
// A Java class's static member class named through a subclass: it is no member of the subclass
// (scalac 3.8.4 refuses the definition too, with a type mismatch).
// expect: 9:10: error: type Double is not a member of Mine
class Mine(a: Int) extends java.awt.Point(a, a)

@main def run(): Unit =
  val d: Mine#Double = new java.awt.geom.Point2D.Double(1.0, 2.0)
  println(d.getX)
