// jars: scala-library
// std: scala-library
// targets: jvm
// A Java class's static member classes are members of its module class, not of the class: inside a
// class below `java.awt.Point`, `Double` and `Float` are `scala`'s, not `Point2D`'s member classes.
class Mine(a: Int) extends java.awt.Point(a, a):
  def half: Double = a / 2.0
  def third: Float = a / 4.0f

@main def run(): Unit =
  val m = new Mine(3)
  println(m.half + m.third)
