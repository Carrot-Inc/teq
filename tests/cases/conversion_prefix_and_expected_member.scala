// Implicit conversions compared under the objects they are found in (skunk's `toTwiddleOpTwo`,
// which `Codec`, `Encoder` and `Decoder` inherit from `TwiddleSyntax[F]`), and an ambiguity between
// two extension conversions settled by the member's expected result (`7.days` for a
// `FiniteDuration` beside zio's `durationInt`), as dotc searches again with the result revealed.
import scala.language.implicitConversions
class Dec[A](val s: String)
class Cod[A](s: String) extends Dec[A](s)
final class Ops[F[_], B](self: F[B]):
  def *:[A](fa: F[A])(using ev: DummyImplicit): F[(A, B)] = self.asInstanceOf[F[(A, B)]]
  def *:[G[x] >: F[x], A](ga: G[A]): G[(A, B)] = ga.asInstanceOf[G[(A, B)]]
trait Syntax[F[_]]:
  implicit def toOps[B](fb: F[B]): Ops[F, B] = new Ops(fb)
object Dec extends Syntax[Dec]
object Cod extends Syntax[Cod]

def take[B](d: Dec[B]): Dec[B] = d
def twiddle(): Unit =
  val c = new Cod[Int]("c")
  println(take(new Dec[String]("d") *: c).s)
  println((new Cod[String]("e") *: c).s)

final case class Fin(days: Int)
final case class Jdur(secs: Long)
object ZioLike:
  final class Syntax(n: Int):
    def days: Jdur = Jdur(n * 86400L)
  implicit def durationInt(n: Int): Syntax = Syntax(n)
object ScalaLike:
  implicit final class DurationInt(private val n: Int):
    def days: Fin = Fin(n)
import ZioLike.*
import ScalaLike.DurationInt
def expire(f: Fin): String = f.toString
def durations(): Unit =
  val a: Fin = 7.days
  val b: Jdur = 2.days
  println((a, b, expire(3.days)))

@main def main(): Unit =
  twiddle()
  durations()
