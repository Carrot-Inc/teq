// jars: scala-library jdkparent-lib
// std: scala-library
// targets: jvm
// Inside a class of the program below a library class below `java.awt.Point`, `Double` and `Float` are
// `scala`'s: the static member classes of the JDK ancestor `Point2D` are no members of the class.
class Mine(a: Int) extends jdkparent.Spot(a, a):
  def half: Double = a / 2.0
  def third: Float = a / 4.0f

@main def run(): Unit =
  val m = new Mine(3)
  println(m.half + m.third)
