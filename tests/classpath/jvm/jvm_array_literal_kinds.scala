// jars: scala-library
// std: lean scala-library
// The kinds of the arrays scalac's `Array.apply` overloads make, printed as the JVM prints
// their elements: the arguments converted to the overload's element type.
import scala.reflect.ClassTag
def ty[T](a: Array[T])(using ct: ClassTag[T]): String = ct.toString + " " + a.mkString(",")
@main def run(): Unit =
  val i = 1
  val s: Short = 2
  println(ty(Array(1, 2, 3)))
  println(ty(Array(i, 2.5)))
  println(ty(Array(1, 2L)))
  println(ty(Array('a', 1)))
  println(ty(Array(s, 'a')))
  println(ty(Array(i, 'a')))
  println(ty(Array(1.5f, i)))
  println(ty(Array(true, false)))
  println(ty(Array((), ())))
  println(ty(Array(1, "x")))
  println(ty(Array[Long](1, 2)))
  println(ty(Array(1.5, 2)))
  val longs: Array[Long] = Array(1, 2)
  println(ty(longs) + " " + longs.getClass.getSimpleName + " " + Array(1, 2.5).getClass.getSimpleName + " " + Array('a', 1).getClass.getSimpleName)
