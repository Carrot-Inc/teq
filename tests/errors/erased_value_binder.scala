// expect: 17:11: error: value x is unusable because it refers to an erased expression in the selector of an inline match
// expect: 18:11: error: value x is unusable because it refers to an erased expression in the selector of an inline match
// expect: 19:11: error: method erasedValue is declared as `erased`, but is in fact used
// expect: 3 errors found
// A binder an erased scrutinee binds is erased and unusable (`InlineReducer.reduceInlineMatch`'s
// `adjustErased`, `cleanupUnusable`), and an `erasedValue` an expansion copies is one as the call
// is (`Erasure.checkNotErased`): each read is rejected, as scalac rejects it.
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
