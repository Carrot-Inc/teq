// jars: scala-library jdkparent-lib
// std: scala-library
// targets: jvm
// The least upper bound of a library class and its JDK parent's own parent (`java.awt.Point`'s
// `Point2D`): the library class's base types hold the JDK parent's, read when the class is completed.
def pick(b: Boolean, p: jdkparent.Spot, q: java.awt.geom.Point2D) = if b then p else q

@main def run(): Unit =
  val p = new jdkparent.Spot(3, 4)
  println(pick(true, p, new java.awt.Point(0, 0)).distance(0.0, 0.0))
