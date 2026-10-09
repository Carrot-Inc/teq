//> using scala 3.8.4
// An alias given is not in scope in its own right-hand side, as in scalac: the search for a
// type it provides goes on to the outer scopes and the implicit scope of the type.

trait Order[A]:
  def compare(a: A, b: A): Int
object Order:
  def fromOrdering[A](using ord: Ordering[A]): Order[A] = new Order[A]:
    def compare(a: A, b: A): Int = ord.compare(a, b)
  given [A](using order: Order[A]): Ordering[A] = Ordering.fromLessThan((a, b) => order.compare(a, b) < 0)

case class Day(n: Int)
object Day:
  given Ordering[Day] = Ordering.by(_.n)

// With the Order-to-Ordering conversion in scope, fromOrdering must not find this very given
// through it; Day's companion Ordering is the one meant.
given Order[Day] = Order.fromOrdering

trait Monad[T]:
  def id: String
class Foo
object Foo:
  given Monad[Foo] with
    def id = "Foo"
opaque type Bar = Foo
object Bar:
  given Monad[Bar] = summon[Monad[Foo]]

@main def main(): Unit =
  println(summon[Order[Day]].compare(Day(1), Day(2)))
  println(summon[Monad[Bar]].id)
