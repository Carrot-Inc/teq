// A lexical extension whose using clause has no instance for the receiver is passed over for
// the next candidate, here the extension of the receiver's companion, as scalac's overload
// resolution drops a candidate that does not type check.

trait Monoid[A]:
  def empty: A
trait Eq[A]:
  def eqv(a: A, b: A): Boolean
given Monoid[Int] with
  def empty = 0
given Eq[Int] with
  def eqv(a: Int, b: Int) = a == b
object Syntax:
  extension [A](a: A)
    def isEmpty(using M: Monoid[A], E: Eq[A]): Boolean = E.eqv(a, M.empty)
import Syntax.*
object Tailwind:
  opaque type Tw = String
  object Tw:
    def of(s: String): Tw = s
    extension (classes: Tw) def isEmpty: Boolean = classes.isEmpty
@main def main(): Unit =
  println(Tailwind.Tw.of("").isEmpty)
  println(Tailwind.Tw.of("a").isEmpty)
  println(0.isEmpty)
  println(List(1).isEmpty)
