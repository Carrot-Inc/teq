// The view into an open variable approximates the variables its bounds hold however deep, cutting
// only a variable met again (`ProtoTypes.wildApprox`'s `seen`): `E <: List[D]` down to `A <: Int`
// keeps `Int`, so of two views from `Raw` the one ending in `Int` is taken, for a for-comprehension's
// lambda and a written one alike, as scalac takes it.
package foryieldviewdepth

import scala.language.implicitConversions

final class Task[A](val value: A):
  def map[B](f: A => B): Task[B] = new Task(f(value))
final case class Box[A](value: A)
final case class Raw(n: Int)
implicit def good(r: Raw): Box[List[List[List[List[Int]]]]] = Box(List(List(List(List(r.n)))))
implicit def bad(r: Raw): Box[List[List[List[List[String]]]]] = Box(List(List(List(List("bad")))))
def wrap[A <: Int, B <: List[A], C <: List[B], D <: List[C], E <: List[D]](t: Task[Box[E]]): String = t.value.value.toString

@main def main(): Unit =
  println(wrap(for x <- new Task(1) yield Raw(x)))
  println(wrap(new Task(2).map(x => Raw(x))))
