// expect: 13:65: error: type mismatch: found Int, required String
// A selector `conv as _` leaves the value's conversion out of the wildcard import (scalac: E007).
import scala.language.implicitConversions
class Dsl(n: Int):
  def root: String = s"root $n"
  def size(s: String): String = s"len ${s.length}"
  implicit def conv(i: Int): String = "x" * i
object Build:
  def use[A](n: Int)(f: Dsl => A): A = f(new Dsl(n))
@main def main(): Unit =


  println(Build.use(5) { dsl => import dsl.{conv as _, *}; size(3) })
