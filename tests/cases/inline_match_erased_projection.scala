// A field of an erased scrutinee that a constructor application gives with an elideable argument
// is that argument (`InlineReducer.reduceProjection`, then `constToLiteral`), which `adjustErased`
// finds free of the erased value: its binder is usable, as scalac reduces it.
import scala.compiletime.erasedValue
case class Pair(a: Int, b: String)
inline def second[T]: Int = inline (erasedValue[T], 1) match
  case (_, x) => x
inline def first[T]: String = inline (erasedValue[T], "s") match
  case (_, s: String) => s
inline def named: String = inline Pair(1, erasedValue[String]) match
  case Pair(n, _) => s"n=$n"
@main def run(): Unit =
  println(second[Int])
  println(first[Int])
  println(named)
