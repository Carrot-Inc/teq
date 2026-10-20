// lila's GameRepo: `_.fold(HoldAlert.emptyMap) { doc => chess.ByColor(holdAlertOf(doc, _)) }`.
// scalachess overloads ByColor.apply with `[A](f: Color => A): ByColor[A]` and
// `[F[_], A](f: Color => F[A])(using Applicative[F]): F[ByColor[A]]`. The lambda's expected
// result is fold's type variable B, bounded by ByColor[Option[HoldAlert]]; scalac's
// resolveOverloaded (adaptByResult: constrainResult against pt) drops the Applicative
// alternative, whose result does not fit. teq picks it when pt is a type variable (with a
// concrete expected type, `val x: ByColor[Option[Int]] = ByColor(alert("d", _))`, it does not).
trait Applicative[F[_]]:
  def pure[A](a: A): F[A]
given Applicative[Option] with
  def pure[A](a: A) = Some(a)
enum Color:
  case White, Black
case class ByColor[A](white: A, black: A)
object ByColor:
  def apply[A](f: Color => A): ByColor[A] = ByColor(f(Color.White), f(Color.Black))
  def apply[F[_], A](f: Color => F[A])(using ap: Applicative[F]): F[ByColor[A]] =
    ap.pure(ByColor(???, ???))

def alert(d: String, c: Color): Option[Int] = if c == Color.White then Some(d.length) else None
val empty: ByColor[Option[Int]] = ByColor(None, None)

def holdAlerts(doc: Option[String]): ByColor[Option[Int]] =
  doc.fold(empty)(d => ByColor(alert(d, _)))

def id[X](x: X): X = x
def direct: ByColor[Option[Int]] = id(ByColor(alert("abc", _)))

@main def run() =
  println(holdAlerts(Some("ab")))
  println(holdAlerts(None))
  println(direct)
