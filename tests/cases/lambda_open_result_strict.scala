// Solving an enclosing variable keeps an invariant inner one open: `delay`'s `A` is `Item[T]` with `T`
// open (`ConstraintHandling.instanceType` takes the full bound and repairs levels, nothing more), so
// `::`'s expected type fixes `T` to `Option[String]`; the argument is evaluated once, strict or by name.
package lambdaopenresultstrict

final case class Item[T](title: String, value: T, disabled: Boolean = false)
def delay[A](a: A): List[A] = List(a)
def delayed[A](a: => A): List[A] = List(a)

@main def main(): Unit =
  var n = 0
  val xs: List[Item[Option[String]]] =
    Item[Option[String]]("none", None) :: delay({ n += 1; Item("x", Some("x")) })
  println((xs.map(_.value), n))
  val ys: List[Item[Option[String]]] =
    Item[Option[String]]("none", None) :: delayed({ n += 1; Item("y", Some("y")) })
  println((ys.map(_.value), n))
