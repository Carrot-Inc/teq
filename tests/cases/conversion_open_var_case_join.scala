// A conversion leaves a variable its result holds nonvariantly open (cats' `catsSyntaxOptionId(a):
// OptionIdOps[?A]`), and the member it serves interpolates it, as dotty's `interpolateTypeVars` does at
// the selection: `.some` holds it covariantly (`Option[?A]`) and instantiates it from below. The cases of
// a match then join to `Option[T]` with a generic `none` after them (lila's `Challenger`s, a notification's
// expiry, a broadcast id read from a path), not to `Option[Any]`; `pure[F]` keeps it open (`F[?A]`).
import scala.language.implicitConversions

final class OptionIdOps[A](private val a: A) extends AnyVal:
  def some: Option[A] = Some(a)
def none[A]: Option[A] = Option.empty[A]
implicit def catsSyntaxOptionId[A](a: A): OptionIdOps[A] = new OptionIdOps(a)

trait Zero[A]:
  def zero: A
given Zero[String] with
  def zero = ""
extension [A](self: Option[A]) def so[B](f: A => B)(using z: Zero[B]): B = self.fold(z.zero)(f)

sealed trait Challenger
case class Registered(id: Int) extends Challenger
case class Anonymous(sid: String) extends Challenger
case object Open extends Challenger
def describe(c: Challenger): String = c match
  case Registered(id) => s"registered $id"
  case Anonymous(sid) => s"anonymous $sid"
  case Open => "open"

def challenger(user: Option[Int], sid: Option[String], open: Boolean): String =
  (user, sid)
    .match
      case (Some(u), _) => Registered(u).some
      case (_, Some(s)) => Anonymous(s).some
      case _ if open => Open.some
      case _ => none
    .so: c =>
      describe(c)

def expiry(kind: Int): Option[Long] =
  val expiresIn = kind match
    case 1 => 6L.some
    case 2 => 12L.some
    case _ => none
  expiresIn.map(_ * 3600)

def broadcastId(path: String): Option[Int] =
  for
    p <- Option(path)
    id <- p.split('/').filter(_.nonEmpty) match
      case Array("broadcast", id, "edit") => id.some
      case Array("broadcast", _, id) => id.some
      case _ => none
  yield id.length

@main def main(): Unit =
  println(challenger(Some(7), None, false))
  println(challenger(None, Some("s1"), false))
  println(challenger(None, None, true))
  println(challenger(None, None, false).isEmpty)
  println(expiry(1))
  println(expiry(3))
  println(broadcastId("/broadcast/abc/edit"))
  println(broadcastId("/broadcast/x/defgh"))
  println(broadcastId("/other"))
