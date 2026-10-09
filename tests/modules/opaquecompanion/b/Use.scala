package ocb

import oca.ShelfKey

trait Tagged

object Use:
  def first(keys: List[ShelfKey]): Option[ShelfKey] = keys.headOption
  def tagged(k: ShelfKey & Tagged): ShelfKey = k
  def all(keys: List[ShelfKey]): List[Int] = keys.map(k => k.value).filter(_ > 0)
  def both(f: ShelfKey => Int, g: ShelfKey => Int): ShelfKey => Int = k => f(k) + g(k)

@main def run(): Unit =
  val keys = List(ShelfKey.of(3), ShelfKey.of(-1))
  println(Use.all(keys))
  println(Use.first(keys).map(_.value))
  println(Use.both(_.value, _.value * 2)(ShelfKey.of(5)))
