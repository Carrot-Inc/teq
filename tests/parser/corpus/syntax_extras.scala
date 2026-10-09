//> using scala 3.8.4
import scala.annotation.tailrec

trait Show[A]:
  def show(a: A): String

final case class Box[A](value: A)
final class Secret private (val code: Int)
object Secret:
  def make(code: Int): Secret = new Secret(code)

infix type or[A, B] = Either[A, B]

final class FnShow[A](f: A => String) extends Show[A]:
  def show(a: A): String = f(a)

given Show[Int] = FnShow(a => s"int $a")

given [A: Show] => Show[Box[A]] = FnShow(b => "box of " + summon[Show[A]].show(b.value))

given listShow: [A] => (inner: Show[A]) => Show[List[A]] =
  FnShow(xs => xs.map(inner.show).mkString("[", ", ", "]"))

given [A] => (inner: Show[A]) => Show[Option[A]]:
  def show(o: Option[A]): String = o.fold("none")(inner.show)

def both[A: {Show, Ordering}](a: A, b: A): String =
  val s = summon[Show[A]]
  if summon[Ordering[A]].lt(a, b) then s.show(a) else s.show(b)

def describe(e: Int or String): String = e match
  case Left(n) => s"number $n"
  case Right(s) => s"text $s"

def size(xs: List[? <: Any]): Int = xs.length

type Align = "left" | "right"
def align(a: Align): String = a

@scala.annotation.tailrec
def count(n: Int, acc: Int): Int = if n == 0 then acc else count(n - 1, acc + 1)

@main def main(): Unit =
  println(summon[Show[Box[Int]]].show(Box(1)))
  println(summon[Show[List[Box[Int]]]].show(List(Box(1), Box(2))))
  println(both(3, 2))
  println(summon[Show[Option[Int]]].show(Some(4)))
  println(describe(Left(1)) + ", " + describe(Right("x")))
  println(size(List(1, "a")))
  println(align("left"))
  println(count(1000, 0))
  println(Secret.make(7).code)
  val pairs = for
    base = 10
    (lo, hi) = (1, 2)
    x <- List(lo, hi)
    y = x + base
  yield y
  println(pairs)
