// jars: scala-library jdkparent-lib
// Two kinds of unit over one library class below a JDK class the std does not declare: four ask a
// member of the JDK parent, four take the least upper bound of the class and the parent's own
// parent (`java.awt.geom.Point2D`), which reads the class's base types.
@main def run(): Unit =
  val p = new jdkparent.Spot(1, 2)
  println(M1.x(p) + M2.x(p) + M3.x(p) + M4.x(p) + List(P5.pick(true, p, p), P6.pick(true, p, p), P7.pick(false, p, p), P8.pick(true, p, p)).size)
