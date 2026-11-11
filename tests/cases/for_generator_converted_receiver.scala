// A generator whose `map` comes from a conversion of the receiver, the receiver's type argument still open: the
// closure's parameter is instantiated from above (`x: Any`), as a written `map(x => ...)`'s is.
import language.implicitConversions

trait Func[F[_]]:
  def map[A, B](fa: F[A])(f: A => B): F[B]

trait Ops[F[_], A]:
  def fa: F[A]
  def func: Func[F]
  def map[B](f: A => B): F[B] = func.map(fa)(f)

implicit def toOps[F[_], A](fa0: F[A])(using f: Func[F]): Ops[F, A] =
  new Ops[F, A]:
    def fa = fa0
    def func = f

class Cell[A]:
  var depth = 0

object Cell:
  given Func[Cell]:
    def map[A, B](fa: Cell[A])(f: A => B): Cell[B] =
      println("map")
      val c = new Cell[B]
      c.depth = fa.depth + 1
      c

@main def main(): Unit =
  val shown = for x <- Cell() yield x.toString.length
  println(shown.depth)
  val written = Cell().map(x => x)
  println(written.depth)
