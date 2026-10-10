// expect: 23:11: error: value x is unusable because it refers to an erased expression in the selector of an inline match
// expect: 24:11: error: value x is unusable because it refers to an erased expression in the selector of an inline match
// expect: 25:11: error: value x is unusable because it refers to an erased expression in the selector of an inline match
// expect: 3 errors found
// What an erased scrutinee's binder reads is checked on the case's body before it reduces any
// further (`cleanupUnusable`), the dropped branch of its `inline if` included, as scalac reports it;
// a field that the erased value gives, or an argument with an effect (which `reduceProjection`
// keeps in place), is read through the scrutinee, erased as it is: scalac rejects the second too,
// in a later phase (E217, the erased scrutinee's right-hand side impure).
import scala.compiletime.erasedValue
object C:
  var n = 0
  def tick(): Int = { n += 1; 6 }
case class Box(n: Int)
inline def dead[T]: Int = inline erasedValue[T] match
  case x: Int => inline if true then 2 else x
  case _ => 0
inline def effect[T]: Int = inline (erasedValue[T], C.tick()) match
  case (_, x) => x
inline def field: Int = inline erasedValue[Box] match
  case Box(x) => x
@main def run(): Unit =
  println(dead[Int])
  println(effect[Int])
  println(field)
