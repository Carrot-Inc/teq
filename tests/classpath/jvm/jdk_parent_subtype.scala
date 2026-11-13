// jars: scala-library jdkparent-lib
// std: scala-library
// targets: jvm
// A library class is a subtype of its JDK parent's own parents (`Point2D` and `Serializable` through
// `java.awt.Point`).
def up(p: jdkparent.Spot): java.awt.geom.Point2D = p
def ser(p: jdkparent.Spot2): java.io.Serializable = p

@main def run(): Unit =
  val p = new jdkparent.Spot(1, 2)
  val q = new jdkparent.Spot2(0, 0)
  println(up(p).distance(q) + List[java.awt.geom.Point2D](p, q).size)
  println(ser(q) == q)
