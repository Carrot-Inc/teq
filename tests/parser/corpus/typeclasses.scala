//> using platform js
trait Show[A]:
  def show(a: A): String

object Show:
  given Show[Int] with
    def show(a: Int): String = s"Int($a)"

  given Show[String] with
    def show(a: String): String = "\"" + a + "\""

  given Show[Boolean] with
    def show(a: Boolean): String = if a then "yes" else "no"

  given listShow[A](using s: Show[A]): Show[List[A]] with
    def show(xs: List[A]): String = xs.map(s.show).mkString("[", ", ", "]")

  given optionShow[A: Show]: Show[Option[A]] with
    def show(o: Option[A]): String = o match
      case Some(v) => "Some(" + summon[Show[A]].show(v) + ")"
      case None => "None"

  given pairShow[A, B](using sa: Show[A], sb: Show[B]): Show[(A, B)] with
    def show(p: (A, B)): String = "<" + sa.show(p._1) + ", " + sb.show(p._2) + ">"

extension [A](a: A)(using s: Show[A])
  def shown: String = s.show(a)

def showAll[A: Show](xs: List[A]): String =
  xs.map(x => x.shown).mkString(" ")

trait Monoid[A]:
  def empty: A
  extension (x: A) def combine(y: A): A

given Monoid[Int] with
  def empty: Int = 0
  extension (x: Int) def combine(y: Int): Int = x + y

given Monoid[String] with
  def empty: String = ""
  extension (x: String) def combine(y: String): String = x + y

def combineAll[A](xs: List[A])(using m: Monoid[A]): A =
  xs.foldLeft(m.empty)((acc, x) => acc.combine(x))

trait Functor[F[_]]:
  extension [A](fa: F[A]) def fmap[B](f: A => B): F[B]

given Functor[List] with
  extension [A](fa: List[A]) def fmap[B](f: A => B): List[B] = fa.map(f)

given Functor[Option] with
  extension [A](fa: Option[A]) def fmap[B](f: A => B): Option[B] = fa.map(f)

def double[F[_]](fa: F[Int])(using functor: Functor[F]): F[Int] =
  fa.fmap(x => x * 2)

trait Monad[F[_]]:
  def pure[A](a: A): F[A]
  def flatMap[A, B](fa: F[A])(f: A => F[B]): F[B]

given Monad[Option] with
  def pure[A](a: A): Option[A] = Some(a)
  def flatMap[A, B](fa: Option[A])(f: A => Option[B]): Option[B] = fa.flatMap(f)

given Monad[List] with
  def pure[A](a: A): List[A] = List(a)
  def flatMap[A, B](fa: List[A])(f: A => List[B]): List[B] = fa.flatMap(f)

def pairUp[F[_], A, B](fa: F[A], fb: F[B])(using m: Monad[F]): F[(A, B)] =
  m.flatMap(fa)(a => m.flatMap(fb)(b => m.pure((a, b))))

case class Meters(value: Double)

given Ordering[Meters] with
  def compare(a: Meters, b: Meters): Int = if a.value < b.value then -1 else if a.value > b.value then 1 else 0

def largest[A](xs: List[A])(using ord: Ordering[A]): A =
  xs.reduce((a, b) => if ord.gt(a, b) then a else b)

@main def run(): Unit =
  println(1.shown)
  println("hi".shown)
  println(List(1, 2, 3).shown)
  println(List(List(true, false), List(true)).shown)
  println(Option(5).shown)
  println((1, "one").shown)
  println(List((1, true), (2, false)).shown)
  println(showAll(List("a", "b")))
  println(combineAll(List(1, 2, 3, 4)))
  println(combineAll(List("x", "y", "z")))
  println(double(List(1, 2, 3)))
  println(double(Option(21)))
  println(pairUp(Option(1), Option("a")))
  println(pairUp(List(1, 2), List("a", "b")))
  println(largest(List(Meters(1.5), Meters(3.5), Meters(2.0))))
  println(List(3, 1, 2).sorted)
  println(List("pear", "apple", "fig").sortBy(s => s.length))
  println(List(1, 2, 3).sum)
  println(List(1.5, 2.5).sum)
  println(List(3, 9, 4).max)
  println(List((2, "b"), (1, "z"), (1, "a")).sorted)
  val explicit = combineAll(List(1, 2))(using summon[Monoid[Int]])
  println(explicit)
