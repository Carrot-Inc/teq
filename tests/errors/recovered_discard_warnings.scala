// teq: --werror
// A value dropped where the variable it was expected to be is defined as `Unit` only after the value was
// typed is told as any dropped value is: a lambda's result behind a chain of `map`s, a comprehension's yield
// (with named and with wildcard binders), a branch, a lambda's result against a declared bound that the
// expected type pins from below. A call's result is dropped and not told.
final class Shelf[+A](val a: A):
  def map[B](f: A => B): Shelf[B] = Shelf(f(a))
  def flatMap[B](f: A => Shelf[B]): Shelf[B] = f(a)
class Tally:
  def bump(): Tally = this
def pinned[A <: Unit](g: () => A): A => Unit = a => ()

object Dropped:
  val tally = Tally()
  def chain(o: Shelf[Int]): Shelf[Unit] = o.flatMap(_ => o.map(_ => 5))
  def named(o: Shelf[Int]): Shelf[Unit] = for { a <- o; b <- o } yield 6
  def wild(o: Shelf[Int]): Shelf[Unit] = for { _ <- o; _ <- o } yield 7
  def branch(o: Shelf[Int]): Shelf[Unit] = o.flatMap(_ => o.map(_ => if o.a > 0 then 8 else 9))
  def call(o: Shelf[Int]): Shelf[Unit] = o.flatMap(_ => o.map(_ => tally.bump()))
  val first: Unit => Unit = pinned(() => 1)
