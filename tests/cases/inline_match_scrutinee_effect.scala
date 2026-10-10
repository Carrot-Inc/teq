// An `inline match` evaluates its scrutinee once, before the selected case, whether a case reads it or not:
// scalac binds it (`InlineReducer.reduceInlineMatch`'s `$scrutineeN`) and drops the unused binding only of an
// elideable one (`Inliner.dropUnusedDefs`): an inline or by-name argument with an effect runs, so does a lazy
// val's initialiser; a literal, a val and `erasedValue` do not.
import scala.compiletime.erasedValue
var n = 0
def tick(): Int = { n += 1; 1 }
lazy val lz: Int = { println("lazy"); 2 }
val v = 3
inline def typeOnly(inline x: Int): Int = inline x match { case _: Int => 7 }
inline def byName(x: => Int): Int = inline x match { case _: Int => 8 }
inline def read(inline x: Int): Int = inline x match { case y: Int => y + 10 }
inline def kind[T]: String = inline erasedValue[T] match
  case _: Int => "int"
  case _ => "other"
@main def run(): Unit =
  println(typeOnly(tick()))
  println(n)
  println(byName(tick()))
  println(n)
  println(read(tick()))
  println(n)
  println(typeOnly(lz))
  println(typeOnly(v) + typeOnly(4))
  println(kind[Int] + kind[String])
