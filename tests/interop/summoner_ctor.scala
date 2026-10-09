// Not Scala: scalac reports a missing argument list for `Codec[Int]` when the `apply` of
// `object Codec` is the case-class constructor; teq reads it as `summon[Codec[Int]]`, which is
// what the summoner overload of a library like zio-json makes of it.
package summonerctor

trait Enc[A]:
  def enc(a: A): String

case class Codec[A](enc: Enc[A], name: String):
  def transform[B](f: A => B, g: B => A): Codec[B] =
    Codec(b => enc.enc(g(b)), name + "'")

object Codec:
  given Codec[Int] = Codec(a => a.toString, "int")
  given Codec[String] = Codec(s => s"\"$s\"", "str")

sealed trait Err
case class AuthError(details: String) extends Err

object AuthError:
  given Codec[AuthError] = Codec[String].transform(d => AuthError(d), _.details)

case class Wrapped(n: Int)
object Wrapped:
  given Codec[Wrapped] = Codec[Int].transform(Wrapped.apply, _.n)

case class Pair[A](a: A, b: A)
object Pair:
  given Codec[Pair[Int]] = Codec(p => s"${p.a},${p.b}", "pair")

class Validated[A](val value: A, val ok: Boolean)
object Validated:
  def apply[A](value: A): Validated[A] = new Validated(value, true)
  given Validated[Int] = new Validated(7, false)

@main def main(): Unit =
  println(Codec[Int].name)
  println(Codec[Int].enc.enc(3))
  println(Codec[AuthError].enc.enc(AuthError("bad")))
  println(Codec[AuthError].name)
  println(Codec[Wrapped].enc.enc(Wrapped(4)))
  println(Codec[Pair[Int]].enc.enc(Pair(1, 2)))
  val c = Codec[String]
  println(c.name)
  println(Validated[Int].value)
  println(Validated(1).ok)
  println(List(3, 1, 2).sorted(using Ordering[Int].reverse))
  println(List(1, 2).map(Pair[Int].apply(_, 0)))
