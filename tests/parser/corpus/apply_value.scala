package applyvalue

case class Zone(id: Int, name: String)
case class Wrap(value: Int)
case class Box[A](value: A)
case class Pair[A, B](left: A, right: B)
case class Curried(a: Int)(b: Int):
  def sum: Int = a + b

enum Sel:
  case All
  case One(id: Int)
  case Two(a: Int, b: String)

enum Tree[+A]:
  case Leaf
  case Node(value: A, children: List[Tree[A]])

case class WithCompanion(x: Int, y: Int)
object WithCompanion:
  val origin: WithCompanion = WithCompanion(0, 0)
  def sum(w: WithCompanion): Int = w.x + w.y

case class OwnApply(n: Int)
object OwnApply:
  def apply(text: String): OwnApply = new OwnApply(text.length)

object Models:
  case class Inner(n: Int)
  object Deep:
    case class Innermost(a: Int, b: Int)

final case class Meta[Id](id: Id, title: String, hidden: Boolean)

extension [A, B](t: (Option[A], Option[B]))
  def mapN[R](f: (A, B) => R): Option[R] =
    t._1.flatMap(a => t._2.map(b => f(a, b)))

extension [A, B, C](t: (Option[A], Option[B], Option[C]))
  def mapN[R](f: (A, B, C) => R): Option[R] =
    for a <- t._1; b <- t._2; c <- t._3 yield f(a, b, c)

def build[A, B, R](a: A, b: B)(f: (A, B) => R): R = f(a, b)
def describe(s: Sel): String = s match
  case Sel.All => "all"
  case Sel.One(id) => s"one $id"
  case Sel.Two(a, b) => s"two $a $b"

@main def main(): Unit =
  val f = Zone.apply
  println(f(1, "a"))
  println(Zone.apply(7, "q"))
  println(List(1, 2).map(Wrap.apply))
  println(build(1, "x")(Zone.apply))
  println((Option(1), Option("n")).mapN(Zone.apply))
  println((Option(1), Option.empty[String]).mapN(Zone.apply))

  println(List(1, 2).map(Sel.One.apply).map(describe))
  println(build(2, "y")(Sel.Two.apply))
  val one = Sel.One.apply
  val sels: List[Sel] = List(one(1), Sel.All)
  println(sels.map(describe))
  val node = Tree.Node.apply[Int]
  println(node(1, List(Tree.Leaf)))
  println((Option(1), Option(List(Tree.Leaf))).mapN(Tree.Node.apply))

  println(List(1, 2).map(Box.apply))
  val g: Int => Box[Int] = Box.apply
  println(g(3))
  println(List(1, 2).map(Box[Int].apply))
  println(build("l", 2.5)(Pair.apply))
  println(build("l", 2)(Pair[String, Int].apply))
  val meta = Meta[Option[Int]].apply
  println(meta(Some(1), "t", false))
  println((Option(Option(2)), Option("u"), Option(true)).mapN(Meta[Option[Int]].apply))

  println(build(5, 6)(WithCompanion.apply))
  println(WithCompanion.sum(WithCompanion.origin))
  println(List("ab", "abc").map(OwnApply.apply))
  println(List(1).map(Models.Inner.apply))
  println(build(1, 2)(Models.Deep.Innermost.apply))
  println(build(3, "z")(applyvalue.Zone.apply))

  val h = Wrap.apply andThen Box.apply
  println(h(4))
  println(Some(1).map(Left.apply))
  println(List(1, 2).map(Some.apply))
  println(List(1, 2).map(Right.apply[String, Int]))

  val curried = Curried.apply
  println(curried(1)(2).sum)

  println(List(3).map(WithCompanion(_, 1)))
  println(List(3).map(Zone(_, "z")))
  println(build(4, "p")(Zone(_, _)))
  val named = Zone(_, "fixed")
  println(named(9))
  val inner = Models.Inner(_)
  println(inner(8))
  println(List(1, 2).map(Sel.Two(_, "s")).map(describe))
