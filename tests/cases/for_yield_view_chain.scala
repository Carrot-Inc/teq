// The view into an open variable keeps the bounds its approximation reaches through another
// variable (`ProtoTypes.wildApprox`): `C <: List[B]` with `B <: A` and `A <: Int` is a list of
// what is below `Int`, the wildcard `A` gives `B` its own upper bound, and `B <: A & Int` is the
// wildcard of the parts' bounds met (`approxAnd`), so of two views from `Raw` the one ending in
// `Int` is taken, for a for-comprehension's lambda and a written one alike, as scalac takes it.
package foryieldviewchain

import scala.language.implicitConversions

final class Task[A](val value: A):
  def map[B](f: A => B): Task[B] = new Task(f(value))
final case class Box[A](value: A)
final case class Raw(n: Int)
implicit def good(r: Raw): Box[List[Int]] = Box(List(r.n))
implicit def bad(r: Raw): Box[List[String]] = Box(List("bad"))
def wrap[A <: Int, B <: A, C <: List[B]](t: Task[Box[C]]): String = t.value.value.toString
def wrapAnd[A <: Int, B <: A & Int, C <: List[B]](t: Task[Box[C]]): String = t.value.value.toString

@main def main(): Unit =
  println(wrap(for x <- new Task(1) yield Raw(x)))
  println(wrap(new Task(2).map(x => Raw(x))))
  println(wrapAnd(for x <- new Task(3) yield Raw(x)))
  println(wrapAnd(new Task(4).map(x => Raw(x))))
