// A given that a by-name request below it asks for again is held in a lazy local, whose
// initialiser reads the implicit definition named as it is.
trait Show[A]:
  def show(a: A): String

case class Tree(v: Int, children: List[Tree])

implicit def `rec$0`: Show[Int] = (a: Int) => s"#$a"
given showList[A](using s: => Show[A]): Show[List[A]] = (as: List[A]) => as.map(s.show).mkString("[", ",", "]")
given showTree(using si: Show[Int], sl: => Show[List[Tree]]): Show[Tree] = (t: Tree) => si.show(t.v) + sl.show(t.children)

@main def main(): Unit =
  println(summon[Show[Tree]].show(Tree(1, List(Tree(2, Nil)))))
