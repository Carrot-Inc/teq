// One line break is allowed in front of a parameter clause.

trait Show[A]:
  def show(a: A): String

given Show[Int] with
  def show(a: Int): String = "#" + a.toString

given listShow[A]
  (using s: Show[A]): Show[List[A]] with
  def show(as: List[A]): String = as.map(s.show).mkString("[", ",", "]")

given [T]
  (using s: Show[T]): Show[Option[T]] with
  def show(o: Option[T]): String = o match
    case Some(v) => "Some " + s.show(v)
    case None => "None"

final class Box[A](val items: List[A]):
  def toKeyed[K: Show](extractKey: A => K)
                      (f: A => String): List[String] =
    items.map(a => summon[Show[K]].show(extractKey(a)) + "=" + f(a))

def pagePanel(className: String = "none")
               (children: String*): String =
  className + ":" + children.length.toString

def sameColumn(a: Int)
(b: Int): Int = a + b

def first
  (a: Int)
  (b: Int)
  (using s: Show[Int]): String = s.show(a * b)

def commented(a: Int) // the next clause follows
             // after a comment line
             (b: Int): Int = a - b

class Pt(val x: Int)
        (val y: Int):
  def sum: Int = x + y

case class Cc(a: Int)
             (b: Int):
  def total: Int = a + b

enum Shape:
  case Rect(w: Int, h: Int)
  case Scaled(base: Int)
             (factor: Int)

trait Api:
  def curried(a: Int)
             (b: Int): Int
  // A result type ends the signature, so the next line is a statement of its own.
  def plain(a: Int): Int
  (1, 2).toString

class Impl(label: String) extends Api:
  def curried(a: Int)
             (b: Int): Int = a * b
  def plain(a: Int): Int = a + 1
  (1 to 2).foreach(i => println(label + i.toString))
  val pair = (label, 0)
  (3 to 4).foreach(i => println(pair._1 + i.toString))

object Registry:
  // An object takes no parameters, so the next line is not one of its clauses.
  object Marker
  (5 to 6).foreach(i => println("registry" + i.toString))
  val ready: Boolean = true

extension (s: String)
  def rep(n: Int)
         (sep: String): String = List.fill(n)(s).mkString(sep)

extension [A](self: A)
         (using s: Show[A])
  def shown: String = s.show(self)
  def twice: String = s.show(self) + s.show(self)

extension [A](self: List[A])
             (using s: Show[A]) def all: String = self.map(s.show).mkString(",")

@main def main(): Unit =
  println(Box(List(1, 2)).toKeyed(_ + 1)(_.toString))
  println(pagePanel()("a", "b"))
  println(pagePanel("k")())
  println(sameColumn(1)(2))
  println(first(3)(4))
  println(commented(9)(4))
  println(Pt(1)(2).sum)
  println(Cc(3)(4).total)
  println(Shape.Rect(1, 2))
  println(Shape.Scaled(2)(3))
  val api: Api = Impl("init")
  println(api.curried(3)(4))
  println(api.plain(1))
  println(Registry.ready)
  println("ab".rep(3)("-"))
  println(1.shown)
  println(2.twice)
  println(List(1, 2).all)
  println(summon[Show[List[Int]]].show(List(1, 2)))
  println(summon[Show[Option[Int]]].show(Some(3)))
