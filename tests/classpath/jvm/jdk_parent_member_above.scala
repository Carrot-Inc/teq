// jars: scala-library jdkparent-lib
// std: scala-library
// targets: jvm
// A member that a library class inherits from its JDK parent's own parent (`Point2D.distance` through
// `java.awt.Point`), asked of the library class and of a class of the program below it.
class Mine(a: Int) extends jdkparent.Spot(a, a)

@main def run(): Unit =
  val p = new jdkparent.Spot(3, 4)
  println(p.distance(0.0, 0.0))
  println(new Mine(1).distance(1.0, 0.0))
