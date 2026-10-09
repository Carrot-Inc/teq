// A lambda body typed against an open result variable sees the bound the expected type put on
// it: an enum case stays the case, an Int result widens to the Long asked for, and the function
// value of an enum case's apply keeps the case type against an expected function type.

final case class V[+A](a: A)
extension [A0, A1](t: (V[A0], V[A1])) def mapN[Z](f: (A0, A1) => Z): V[Z] = V(f(t._1.a, t._2.a))

enum Ch:
  case Email(s: String, t: String)
  case Post(s: String)

def toEmail(a: String, b: String): V[Ch.Email] =
  (V(a), V(b)).mapN: (s, t) =>
    Ch.Email(s, t)

final case class Id(value: Int)

enum E:
  case A(value: Int)
final class Codec[X](val f: X => String)
extension [X](c: Codec[X]) def transform[Y](f: X => Y, g: Y => X): Codec[Y] = Codec(y => c.f(g(y)))
given Codec[E.A] = Codec[Int](_.toString).transform(E.A(_), _.value)

val mk: Int => E.A = E.A.apply

@main def main(): Unit =
  println(toEmail("a", "b"))
  println(toEmail("a", "b").a.t)
  val o: Option[Id] = Some(Id(1))
  val l: Option[Long] = o.map(_.value)
  println(l.map(_ + 1L))
  val d: List[Double] = List(Id(2), Id(3)).map(_.value)
  println(d.map(_ / 4))
  println(summon[Codec[E.A]].f(E.A(3)))
  println(mk(1).value)
