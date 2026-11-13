// jars: scala-library jdkparent-lib
// Eight units that each ask a library class for a member of its JDK parent (`java.awt.Point`'s
// `getX`), first at about the same moment.
@main def run(): Unit =
  val p = new jdkparent.Spot(1, 2)
  println(List(U1.x(p), U2.x(p), U3.x(p), U4.x(p), U5.x(p), U6.x(p), U7.x(p), U8.x(p)).sum)
