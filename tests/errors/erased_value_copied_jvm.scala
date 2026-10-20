// jars: scala-library
// teq: --target jvm --std=scala-library
// expect: 14:11: error: value x is unusable because it refers to an erased expression in the selector of an inline match
// absent: 15:11: error
// expect: 1 error found
// Over scala-library `erasedValue` is a plain method of its package object: a binder over it is
// unusable in the inlining phase, as on the lean library, whose error leaves `Erasure` unrun.
import scala.compiletime.erasedValue
inline def leak[T]: Int = inline erasedValue[T] match
  case x: Int => x
  case _ => 0
inline def make[T]: () => T = () => erasedValue[T]
@main def run(): Unit =
  println(leak[Int])
  println(make[Int]())
