// The least upper bound of applications of one class that differ in an invariant argument:
// dotc keeps their union, and selecting a member joins it to the class over a bounded wildcard
// (`Fr[Op, ? >: Named <: Elem]`, `lubArgs`), whose argument is read at its upper bound.
sealed abstract class Fr[S[_], A] extends Product with Serializable:
  def map[B](f: A => B): Fr[S, B] = Fr.Pure(f(this.asInstanceOf[Fr.Pure[S, A]].a))
  def get: A = this.asInstanceOf[Fr.Pure[S, A]].a
object Fr:
  final case class Pure[S[_], A](a: A) extends Fr[S, A]
trait Op[A]
type IO[A] = Fr[Op, A]
trait Applicative[F[_]]:
  def pure[A](a: A): F[A]
given Applicative[IO] with
  def pure[A](a: A): IO[A] = Fr.Pure(a)
final class IdOps[A](private val a: A) extends AnyVal:
  def pure[F[_]](using F: Applicative[F]): F[A] = F.pure(a)
implicit def idOps[A](a: A): IdOps[A] = new IdOps(a)

enum Elem:
  case Shelves, Picks
  case Named(n: String)

final case class Box[A](a: A)
final case class Pair[+A, B](a: A, b: B)

def three(i: Int): IO[Elem] =
  (i match
    case 0 => Elem.Shelves.pure[IO]
    case 1 => Elem.Picks.pure[IO]
    case _ => Elem.Named("x").pure[IO]
  ).map(e => e)

def two(c: Boolean): String =
  val b = if c then Box(1) else Box("s")
  b.a.toString

def nested(c: Boolean): Int =
  val b = if c then Box(Box(1)) else Box(Box(2L))
  b.a.a.toString.length

def sides(c: Boolean): String =
  val p = if c then Pair(1, "a") else Pair(2L, 'b')
  p.a.toString + p.b.toString

def expected(c: Boolean): Box[Any] =
  if c then Box(1) else Box("s")

def selected(i: Int): String =
  (i match
    case 0 => Elem.Shelves.pure[IO]
    case _ => Elem.Named("y").pure[IO]
  ).get.toString

sealed trait Element
object Element:
  case object Genres extends Element
  case object Picks extends Element
  case object NewArrivals extends Element
  case object MostBorrowed extends Element
  final case class InGenre(genre: String, title: String) extends Element
  final case class Cover(url: String) extends Element
  final case class Notice(text: String) extends Element

type ConnectionIO[A] = Fr[Op, A]
def sixWays(i: Int): ConnectionIO[Element] =
  (i match
    case 0 => Element.Genres.pure[ConnectionIO]
    case 1 => Element.Picks.pure[ConnectionIO]
    case 2 => Element.NewArrivals.pure[ConnectionIO]
    case 3 => Element.MostBorrowed.pure[ConnectionIO]
    case 4 => Element.InGenre("c", "t").pure[ConnectionIO]
    case 5 => Element.Cover("u").pure[ConnectionIO]
    case _ => Element.Notice("b").pure[ConnectionIO]
  ).map(e => e)

@main def main(): Unit =
  println(List(0, 1, 2).map(three))
  println(two(true) + two(false))
  println(nested(true) + nested(false))
  println(sides(true) + sides(false))
  println(expected(true).a.toString + expected(false).a)
  println(selected(0) + selected(1))
  println(List(0, 4, 6).map(sixWays))
