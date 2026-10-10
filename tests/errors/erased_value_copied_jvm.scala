// jars: scala-library
// teq: --target jvm --std=scala-library
// expect: 14:11: error: value x is unusable because it refers to an erased expression in the selector of an inline match
// expect: 15:11: error: method erasedValue is declared as `erased`, but is in fact used
// expect: 2 errors found
// Over scala-library `erasedValue` is a plain method of its package object: its copies in an
// expansion are erased values as the call is, and a binder over it unusable, as on the lean library.
import scala.compiletime.erasedValue
inline def leak[T]: Int = inline erasedValue[T] match
  case x: Int => x
  case _ => 0
inline def make[T]: () => T = () => erasedValue[T]
@main def run(): Unit =
  println(leak[Int])
  println(make[Int]())
