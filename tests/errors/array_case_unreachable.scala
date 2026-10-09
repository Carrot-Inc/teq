// expect: 12:8: warning: unreachable case
// expect: 18:8: warning: unreachable case
// expect: 20:8: warning: unreachable case
// expect: 28:8: warning: unreachable case
// expect: 4 warnings found, errors under --werror
// teq: --target jvm --werror
// On the JVM a test against an array takes the arrays of its type, so a later test against
// an array is unreachable where an earlier one takes every array it does, as the types say:
// the same four that scalac reports (E030), and none of the others.
def bounded(x: Any): Int = x match
  case _: Array[? <: Int] => 1
  case _: Array[Int] => 2
  case _ => 3
def kinds(x: Any): Int = x match
  case _: Array[Int] => 1
  case _: Array[Double] => 2
  case _: Array[String] => 3
  case _: Array[Int] => 4
  case _: Array[?] => 5
  case _: Array[Long] => 6
  case _ => 0
def references(x: Any): Int = x match
  case _: Array[AnyRef] => 1
  case _: Array[String] => 2
  case _ => 0
def below(x: Any): Int = x match
  case _: Array[? <: AnyRef] => 1
  case _: Array[String] => 2
  case _ => 0
def either(x: Array[Int] | Array[String]): Int = x match
  case _: Array[Int] => 1
  case _: Array[String] => 2
def tops(x: Any): Int = x match
  case _: Array[Any] => 1
  case _: Array[AnyVal] => 2
  case _: Array[AnyRef] => 3
  case _ => 0
def nested(x: Any): Int = x match
  case _: Array[Array[?]] => 1
  case _: Array[Array[Int]] => 2
  case _ => 0
@main def run(): Unit = println(bounded(Array(1)) + kinds(Array(1L)) + references(1) + below(1) + either(Array(1)) + tops(1) + nested(1))
