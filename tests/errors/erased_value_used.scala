// expect: 12:11: error: method erasedValue is declared as `erased`, but is in fact used
// expect: 13:16: error: method erasedValue is declared as `erased`, but is in fact used
// expect: 2 errors found
// An `erasedValue` the trees keep as a value is used, which scalac's `Erasure.checkNotErased` rejects, at the call or
// at the inline call whose expansion keeps it; one an `inline match` tests, its scrutinee or a part of it, is no value.
import scala.compiletime.erasedValue
inline def zero[T]: T = erasedValue[T]
inline def kind[T]: String = inline erasedValue[T] match { case _: Int => "int"; case _ => "other" }
inline def both[A, B]: String = inline (erasedValue[A], erasedValue[B]) match { case _: (Int, Int) => "ii"; case _ => "other" }
@main def run(): Unit =
  println(kind[Int] + both[Int, Int])
  println(erasedValue[Int])
  val z: Int = zero[Int]
  println(z)
