// expect: 17:11: error: value x is unusable because it refers to an erased expression in the selector of an inline match
// expect: 18:11: error: value x is unusable because it refers to an erased expression in the selector of an inline match
// absent: 19:11: error
// expect: 2 errors found
// A binder an erased scrutinee binds is unusable (`InlineReducer`'s `adjustErased`, `cleanupUnusable`),
// an error of the inlining phase, after which `Erasure` (`checkNotErased` of the copied `erasedValue`)
// does not run. scalac reports the tuple's binder (18) in `Erasure` instead (DIFFERENCES.md).
import scala.compiletime.erasedValue
inline def leak[T]: Int = inline erasedValue[T] match
  case x: Int => x
  case _ => 0
inline def leakTuple[T]: Int = inline (erasedValue[T], 1) match
  case (x: Int, _) => x
  case _ => 0
inline def make[T]: () => T = () => erasedValue[T]
@main def run(): Unit =
  println(leak[Int])
  println(leakTuple[Int])
  println(make[Int]())
