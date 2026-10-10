// A ClassTag's default `newArray` makes its array as `java.lang.reflect.Array.newInstance` does:
// filled with the class's zero, and none of `void`, which throws `IllegalArgumentException`;
// `ClassTag.Unit` makes its arrays itself.
import scala.reflect.ClassTag

class T extends ClassTag[Unit]:
  def runtimeClass: Class[?] = classOf[Unit]
class I extends ClassTag[Int]:
  def runtimeClass: Class[?] = classOf[Int]

def make[A](n: Int)(using ClassTag[A]): Array[A] = new Array[A](n)

@main def run(): Unit =
  try println(new T().newArray(1).length) catch case _: IllegalArgumentException => println("illegal")
  try println(make[Unit](1)(using new T).length) catch case _: IllegalArgumentException => println("illegal make")
  println(ClassTag.Unit.newArray(2).length)
  println(new I().newArray(2).toList)
  println(make[Int](2)(using new I).toList)
