//> using dep com.lihaoyi::sourcecode:0.4.2
// jars: sourcecode
// Anonymous givens for which scalac makes up the same name: they are overloads there.
package demo.anonymous

trait TextKey[T]:
  def value(t: T): String
trait LongKey[T]:
  def value(t: T): Long

trait Eq[T]:
  def eqv(a: T, b: T): Boolean
object Eq:
  def by[T, K](f: T => K): Eq[T] = ByKey(f)
  final class ByKey[T, K](f: T => K) extends Eq[T]:
    def eqv(a: T, b: T): Boolean = f(a) == f(b)

trait Show[T]:
  def show(t: T): String

final case class UserId(v: Long)
object UserId:
  given LongKey[UserId] with
    def value(t: UserId): Long = t.v

final case class Slug(s: String)
object Slug:
  given TextKey[Slug] with
    def value(t: Slug): String = t.s

def name(using n: sourcecode.Name): String = n.value

object Blanket:
  given [T: TextKey] => Eq[T] = Eq.by(summon[TextKey[T]].value)
  given [T: LongKey] => Eq[T] = Eq.by(summon[LongKey[T]].value)
  given [K: TextKey, V] => Eq[Map[K, V]] = Eq.by(_.size)
  given [K: LongKey, V] => Eq[Map[K, V]] = Eq.by(_.size + 1)
  given [T] => (id: TextKey[T]) => Show[T] = StringShow(id)
  given [T] => (id: LongKey[T]) => Show[T] = LongShow(id)

final class StringShow[T](id: TextKey[T]) extends Show[T]:
  def show(t: T): String = "string id " + id.value(t)
final class LongShow[T](id: LongKey[T]) extends Show[T]:
  def show(t: T): String = "long id " + id.value(t)

import Blanket.given

given [T: TextKey] => Ordering[T] =
  println(name)
  Ordering.by(summon[TextKey[T]].value)
given [T: LongKey] => Ordering[T] =
  println(name)
  Ordering.by(summon[LongKey[T]].value)

def same[A](a: A, b: A)(using e: Eq[A]): Boolean = e.eqv(a, b)
def show[A](a: A)(using s: Show[A]): String = s.show(a)

@main def main(): Unit =
  println(same(UserId(1), UserId(1)))
  println(same(UserId(1), UserId(2)))
  println(same(Slug("a"), Slug("a")))
  println(same(Slug("a"), Slug("b")))
  println(same(Map(UserId(1) -> 1), Map(UserId(2) -> 2)))
  println(same(Map(Slug("a") -> 1), Map(Slug("b") -> 2)))
  println(show(UserId(4)))
  println(show(Slug("four")))
  println(List(UserId(3), UserId(1)).sorted)
  println(List(Slug("b"), Slug("a")).sorted)
