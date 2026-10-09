// Type classes: givens with parameters, extension methods, context bounds, a derived instance.
trait Show[A]:
  def show(a: A): String

object Show:
  given Show[Int] with
    def show(a: Int): String = a.toString
  given Show[String] with
    def show(a: String): String = "\"" + a + "\""
  given [A](using s: Show[A]): Show[List[A]] with
    def show(as: List[A]): String = as.map(s.show).mkString("[", ", ", "]")
  given [A](using s: Show[A]): Show[Option[A]] with
    def show(o: Option[A]): String = o match
      case Some(a) => s"Some(${s.show(a)})"
      case None => "None"

trait Monoid[A]:
  def empty: A
  extension (a: A) def |+|(b: A): A

given Monoid[Int] with
  def empty: Int = 0
  extension (a: Int) def |+|(b: Int): Int = a + b

given Monoid[String] with
  def empty: String = ""
  extension (a: String) def |+|(b: String): String = a + b

extension [A](a: A)(using s: Show[A]) def shown: String = s.show(a)

def combineAll[A: Monoid](as: List[A]): A =
  as.foldLeft(summon[Monoid[A]].empty)(_ |+| _)

case class Pair(left: Int, right: String) derives CanEqual

given Show[Pair] with
  def show(p: Pair): String = s"${p.left.shown} ~ ${p.right.shown}"

@main def run(): Unit =
  println(List(1, 2, 3).shown)
  println(Option("x").shown)
  println(List(Option(1), None).shown)
  println(combineAll(List(1, 2, 3)))
  println(combineAll(List("a", "b")))
  println(Pair(1, "one").shown)
