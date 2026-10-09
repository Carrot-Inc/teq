// Blanket givens for `T` and for `T or Other` side by side: the union in the head of a given is
// matched member by member, so `T` stands for the enum alone.
package demo.unions

trait Enumerated[T]:
  def entryName(t: T): String

trait Enc[A]:
  def encode(a: A): String
  def contramap[B](f: B => A): Enc[B] = Enc.Contra(this, f)
object Enc:
  final class Contra[A, B](self: Enc[A], f: B => A) extends Enc[B]:
    def encode(b: B): String = self.encode(f(b))
  final class Str extends Enc[String]:
    def encode(a: String): String = "\"" + a + "\""
  given string: Enc[String] = Str()
  given option[A](using e: Enc[A]): Enc[Option[A]] = string.contramap(o => o.fold("null")(e.encode))

trait Eq[A]:
  def eqv(a: A, b: A): Boolean
object Eq:
  def by[A, B](f: A => B): Eq[A] = By(f)
  final class By[A, B](f: A => B) extends Eq[A]:
    def eqv(a: A, b: A): Boolean = f(a) == f(b)

object Shared:
  infix type or[A, B] = A | B
  final case class Other(value: String)

  def nameOrOther[T](v: T or Other)(using m: Enumerated[T]): String = v match
    case o: Other => o.value
    case t => m.entryName(t.asInstanceOf[T])

  given enumEnc: [T: Enumerated] => Enc[T] = Enc.string.contramap(summon[Enumerated[T]].entryName)
  given enumOrOtherEnc: [T: Enumerated] => Enc[T or Other] = Enc.string.contramap(nameOrOther)
  given enumEq: [T: Enumerated] => Eq[T] = Eq.by(summon[Enumerated[T]].entryName)
  given enumOrOtherEq: [T: Enumerated] => Eq[T or Other] = Eq.by(nameOrOther)

import Shared.{*, given}

enum Color:
  case Red, Green
object Color:
  given Enumerated[Color] with
    def entryName(t: Color): String = t.toString.toLowerCase

extension [A](a: A)(using e: Enc[A]) def toJson: String = e.encode(a)
extension [A](a: A)(using e: Eq[A]) def ===(b: A): Boolean = e.eqv(a, b)

final case class Pref(color: Color or Other, fallback: Option[Color or Other])

@main def main(): Unit =
  val p = Pref(Color.Red, Some(Other("teal")))
  println(Color.Green.toJson)
  println(p.color.toJson)
  println(p.fallback.toJson)
  println(summon[Enc[Color or Other]].encode(Other("x")))
  println(p.color === Color.Red)
  println(p.color === Other("red"))
  println(Color.Red === Color.Green)
