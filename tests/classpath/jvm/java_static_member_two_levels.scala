// jars: scala-library
// std: scala-library
// targets: jvm
// Two levels below the Java class: a class below a class of the program below `java.awt.Point`.
class Base(a: Int) extends java.awt.Point(a, a)
class Mine(a: Int) extends Base(a):
  def half: Double = a / 2.0

@main def run(): Unit =
  println(new Mine(3).half)
