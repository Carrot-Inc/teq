// jars: scala-library javafix
// std: scala-library
// targets: jvm
// A parameter whose type is the intersection of a Java class's static member class and its inner class erases to
// the static member class, which scalac 3.8.4's method takes (`public int Ties.a(fix.Outer$Nested)`).
class Ties:
  def a(x: fix.Outer.Nested & fix.Outer[String]#Inner[Int]): Int = 1

@main def run(): Unit =
  println(classOf[Ties].getDeclaredMethods.find(_.getName == "a").get.toString)
  println(new Ties().a(null))
