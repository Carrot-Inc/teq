//> using scala 3.8.4

// A by-name using parameter is passed unevaluated, and a request that stands below one may be
// answered by the instance under construction: Show[Tree] needs Show[List[Tree]], which needs
// Show[Tree] again, and the knot is tied through the by-name parameter of showTree.
trait Show[A] { def show(a: A): String }
case class Tree(children: List[Tree])
given showTree(using s: => Show[List[Tree]]): Show[Tree] with
  def show(t: Tree) = "T(" + s.show(t.children) + ")"
given showList[A](using s: Show[A]): Show[List[A]] with
  def show(xs: List[A]) = xs.map(s.show).mkString(",")

given showInt: Show[Int] with { def show(a: Int) = a.toString }
given showOpt[A](using s: => Show[A]): Show[Option[A]] with
  def show(o: Option[A]) = o.map(s.show).getOrElse("-")

def render[A](a: A)(using s: => Show[A]): String = s.show(a)

// A by-name class parameter is evaluated at each use.
class Lazy(x: => Int):
  def get = x
  def twice = x + x

// Without a by-name parameter in the chain the recursion diverges as in scalac: the fallback
// instance is what remains.
trait Enc[A] { def n: String }
given encList[A](using e: Enc[A]): Enc[List[A]] with { def n = "list of " + e.n }
given encAny[A]: Enc[A] with { def n = "any" }

@main def main(): Unit =
  println(summon[Show[Tree]].show(Tree(List(Tree(Nil), Tree(List(Tree(Nil)))))))
  println(summon[Show[Option[Int]]].show(Some(1)))
  println(render(Option(2)))
  println(render(Option.empty[Int]))
  println(summon[Enc[List[Int]]].n)
  var n = 0
  val l = Lazy({ n += 1; n })
  println(l.get)
  println(l.twice)
