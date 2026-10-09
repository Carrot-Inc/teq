// A guard of a `summonFrom` case reduces as an inline match's does: a constant `false` gives way
// to the next case, a constant `true` keeps the case; a guard on an inline parameter's constant
// argument folds where the body expands.
import scala.compiletime.summonFrom
trait Show[A]:
  def show(a: A): String
given Show[Int] with
  def show(a: Int): String = "int " + a
inline def describe[A](a: A, inline verbose: Boolean): String = summonFrom {
  case s: Show[A] if verbose => "verbose " + s.show(a)
  case s: Show[A] => s.show(a)
  case _ => "no show"
}
inline def first: String = summonFrom {
  case _: Show[Int] if false => "never"
  case _: Show[String] if true => "string"
  case _: Show[Int] if true => "int"
  case _ => "none"
}
@main def run(): Unit =
  println(describe(1, true))
  println(describe(2, false))
  println(describe("x", true))
  println(first)
