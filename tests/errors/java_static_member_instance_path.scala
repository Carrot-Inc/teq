// jars: scala-library
// teq: --target jvm
// A Java class's static member class is no member of an instance: named on `this`, on a qualified `this`
// and on a value's path it is refused, as scalac 3.8.4 refuses each (`type Double is not a member of ...`).
// expect: 10:12: error: type Double is not a member of Mine.this
// expect: 11:13: error: type Float is not a member of Mine.this
// expect: 15:10: error: type Double is not a member of (p : Point)
// expect: 3 errors found
class Mine(a: Int) extends java.awt.Point(a, a):
  def far: this.Double = new java.awt.geom.Point2D.Double(1.0, 2.0)
  def near: Mine.this.Float = new java.awt.geom.Point2D.Float(1.0f, 2.0f)

@main def run(): Unit =
  val p = new java.awt.Point(1, 2)
  val d: p.Double = new java.awt.geom.Point2D.Double(1.0, 2.0)
  println(d.getX)
