// expect: 14:10: warning: the type test for Array[Q.this.E] cannot be checked at runtime
// expect: 17:8: warning: the type test for Array[T] cannot be checked at runtime
// expect: 20:8: warning: the type test for Array[List[String]] cannot be checked at runtime
// expect: 23:8: warning: the type test for Array[Array[List[Int]]] cannot be checked at runtime
// expect: 26:8: warning: the type test for Array[Option[Int]] cannot be checked at runtime
// expect: 5 warnings found, errors under --werror
// teq: --target jvm --werror
// On the JVM an array carries the class of its component, so a test against an array type is
// checked as far as the element's is: not for a type parameter or an abstract type, and not
// for the type arguments of the element's class. scalac warns of the same five (E092).
trait Q:
  type E
  def member(x: Any): Int = x match
    case _: Array[E] => 1
    case _ => 0
def parameter[T](x: Any): Int = x match
  case _: Array[T] => 1
  case _ => 0
def arguments(x: Any): Int = x match
  case _: Array[List[String]] => 1
  case _ => 0
def nested(x: Any): Int = x match
  case _: Array[Array[List[Int]]] => 1
  case _ => 0
def option(x: Any): Int = x match
  case _: Array[Option[Int]] => 1
  case _ => 0
def checked(x: Any): Int = x match
  case _: Array[List[?]] => 1
  case _: Array[Array[Int]] => 2
  case _: Array[Int | String] => 3
  case _: Array[? <: Int] => 4
  case _: Array[Double] => 5
  case _: Array[String] => 6
  case _: Array[Any] => 7
  case _ => 0
@main def run(): Unit = println(parameter[Int](Array(1)) + arguments(1) + nested(2) + option(3) + checked(Array("s")))
