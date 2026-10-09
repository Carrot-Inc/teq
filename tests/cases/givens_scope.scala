// Givens found in the companion of a type argument that was inferred, directly and through the
// using parameters of other givens.
package demo.scope

trait Eq[A]:
  def eqv(a: A, b: A): Boolean

object Eq:
  def by[A, B](f: A => B)(using e: Eq[B]): Eq[A] = ByKey(f, e)
  def universal[A]: Eq[A] = Universal()
  final class ByKey[A, B](f: A => B, e: Eq[B]) extends Eq[A]:
    def eqv(a: A, b: A): Boolean = e.eqv(f(a), f(b))
  final class Universal[A] extends Eq[A]:
    def eqv(a: A, b: A): Boolean = a == b
  given Eq[Int] = universal
  given Eq[Long] = universal
  given Eq[String] = universal
  given [A](using e: Eq[A]): Eq[Option[A]] with
    def eqv(a: Option[A], b: Option[A]): Boolean = (a, b) match
      case (Some(x), Some(y)) => e.eqv(x, y)
      case (None, None) => true
      case _ => false
  given [A](using e: Eq[A]): Eq[List[A]] with
    def eqv(a: List[A], b: List[A]): Boolean =
      a.length == b.length && a.zip(b).forall((x, y) => e.eqv(x, y))

trait Show[A]:
  def show(a: A): String

object Show:
  given Show[Int] with
    def show(a: Int): String = a.toString
  given [A](using s: Show[A]): Show[Option[A]] with
    def show(a: Option[A]): String = a match
      case Some(x) => "Some(" + s.show(x) + ")"
      case None => "None"

trait Order[A]:
  def compare(a: A, b: A): Int

object Order:
  def by[A, B](f: A => B)(using o: Order[B]): Order[A] = ByKey(f, o)
  final class ByKey[A, B](f: A => B, o: Order[B]) extends Order[A]:
    def compare(a: A, b: A): Int = o.compare(f(a), f(b))
  given Order[Long] with
    def compare(a: Long, b: Long): Int = if a < b then -1 else if a > b then 1 else 0
  given Order[String] with
    def compare(a: String, b: String): Int = a.compareTo(b)

trait Encoder[A]:
  def encode(a: A): String
  def contramap[B](f: B => A): Encoder[B] = Encoder.Contramapped(this, f)

object Encoder:
  final class Contramapped[A, B](self: Encoder[A], f: B => A) extends Encoder[B]:
    def encode(b: B): String = self.encode(f(b))
  given string: Encoder[String] with
    def encode(a: String): String = "\"" + a + "\""
  given long: Encoder[Long] with
    def encode(a: Long): String = a.toString
  given [A](using e: Encoder[A]): Encoder[List[A]] with
    def encode(a: List[A]): String = a.map(e.encode).mkString("[", ",", "]")
  given [A](using e: Encoder[A]): Encoder[Option[A]] with
    def encode(a: Option[A]): String = a match
      case Some(x) => e.encode(x)
      case None => "null"

trait TextKey[T]:
  def value(t: T): String

trait LongKey[T]:
  def value(t: T): Long

trait Enumerated[T]:
  def entryName(t: T): String

final class OrderOrdering[A](o: Order[A]) extends Ordering[A]:
  def compare(a: A, b: A): Int = o.compare(a, b)

object Blanket:
  given [T: TextKey] => Eq[T] = Eq.by(summon[TextKey[T]].value)
  given [T: LongKey] => Eq[T] = Eq.by(summon[LongKey[T]].value)
  given [T: Enumerated] => Eq[T] = Eq.universal
  given [T: TextKey] => Order[T] = Order.by(summon[TextKey[T]].value)
  given [T: LongKey] => Order[T] = Order.by(summon[LongKey[T]].value)
  given [T: TextKey] => Encoder[T] = Encoder.string.contramap(summon[TextKey[T]].value)
  given [T: LongKey] => Encoder[T] = Encoder.long.contramap(summon[LongKey[T]].value)
  given [T: Enumerated] => Encoder[T] = Encoder.string.contramap(summon[Enumerated[T]].entryName)
  given [A] => (o: Order[A]) => Ordering[A] = OrderOrdering(o)

import Blanket.given

extension [A](a: A)(using e: Eq[A])
  def ===(b: A): Boolean = e.eqv(a, b)
  def =!=(b: A): Boolean = !e.eqv(a, b)

extension [A](a: A)(using s: Show[A])
  def show: String = s.show(a)

extension [A](a: A)(using e: Encoder[A])
  def toJson: String = e.encode(a)

final case class UserId(value: Long)
object UserId:
  given LongKey[UserId] with
    def value(t: UserId): Long = t.value

final case class Slug(value: String)
object Slug:
  given TextKey[Slug] with
    def value(t: Slug): String = t.value

enum Color:
  case Red, Green
object Color:
  given Enumerated[Color] with
    def entryName(t: Color): String = t.toString.toLowerCase

final case class Zone(id: UserId, title: String)
object Zone:
  given Eq[Zone] = Eq.by(_.title)
  given Show[Zone] with
    def show(z: Zone): String = "Zone " + z.title
  given Encoder[Zone] = Encoder.string.contramap(_.title)

sealed trait Shape
object Shape:
  final case class Circle(r: Int) extends Shape
  final case class Square(side: Int) extends Shape
  given Eq[Shape] = Eq.universal
  given Show[Shape] with
    def show(s: Shape): String = "shape " + s.toString

object Models:
  final case class Nested(n: Int)
  given Show[Nested] with
    def show(n: Nested): String = "nested " + n.n

def render[A](a: A)(using s: Show[A]): String = s.show(a)
def same[A](a: A, b: A)(using e: Eq[A]): Boolean = e.eqv(a, b)
def widened[A, B >: A](a: A)(using s: Show[B]): String = s.show(a)
def post[Req: Encoder](path: String, payload: Option[Req]): String = path + " " + payload.toJson

@main def main(): Unit =
  println(render(Zone(UserId(1), "north")))
  println(render(Option(3)))
  println(render(Models.Nested(4)))
  println(same(Zone(UserId(1), "a"), Zone(UserId(2), "a")))
  println(same(Shape.Circle(1), Shape.Square(1)))
  println(widened(Shape.Circle(2)))

  println(UserId(1) === UserId(1))
  println(UserId(1) =!= UserId(1))
  println(Slug("a") === Slug("b"))
  println(Color.Red === Color.Green)
  println(Zone(UserId(1), "z") === Zone(UserId(9), "z"))
  println(1 === 1)
  println("a" =!= "b")
  println(Option(UserId(1)) === Option(UserId(1)))
  println(List(Slug("a"), Slug("b")) === List(Slug("a"), Slug("c")))
  val shape: Shape = Shape.Circle(1)
  println(shape === Shape.Circle(1))
  println(shape.show)
  println(Zone(UserId(1), "south").show)

  println(UserId(5).toJson)
  println(Slug("s").toJson)
  println(Color.Green.toJson)
  println(List(UserId(1), UserId(2)).toJson)
  println(Option(Zone(UserId(1), "zone")).toJson)
  println(post("/users", Some(UserId(7))))
  println(post("/zones", Some(List(Zone(UserId(1), "a"), Zone(UserId(2), "b")))))
  println(post("/slugs", Option.empty[Slug]))

  println(List(UserId(3), UserId(1), UserId(2)).sorted)
  println(List(Slug("b"), Slug("a")).sorted)
  println(List(Slug("b"), Slug("a"), Slug("c")).max)
  println(List(UserId(3), UserId(1)).sortBy(u => Slug(u.value.toString)))
