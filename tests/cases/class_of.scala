// classOf[T] is the Class of a class, a trait, a builtin or an array, the same object getClass
// gives an instance; Java's getName() takes its parentheses or none. An array class is left out:
// teq's arrays have no element class at run time.
class Point(val x: Int)
object Point
trait Shape
@main def run(): Unit =
  println(classOf[Point].getName)
  println(classOf[Point].getSimpleName)
  println(classOf[Int].getName)
  println(classOf[String].getName)
  println(classOf[Shape].getName)
  println(classOf[Point].isInstance(Point(1)))
  println(classOf[Point].isInstance("s"))
  println(classOf[Point] == Point(2).getClass)
  println(classOf[String].getName())
  println(classOf[Int].getSimpleName())
  println(classOf[Unit].getName)
