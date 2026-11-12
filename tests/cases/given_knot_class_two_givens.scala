// Two plain inline givens, each the other's by-name clause: the inner one's call is kept while the outer one's
// search is open, and its class reads the outer one's lazy local.
// dotty makes the lazy implicit a local of the search's owner (`SearchRoot.linkBynameImplicit`) and defines it
// around the outermost search's result (`emitDictionary`), so a class anywhere in the result captures it.
trait Show[A]:
  def show(a: A, n: Int): String
final case class Leaf(n: Int)
final case class Tree(n: Int)
object Show:
  inline given leaf(using t: => Show[Tree]): Show[Leaf] = new Show[Leaf]:
    def show(a: Leaf, n: Int): String = if n > 3 then "deep" else "leaf:" + t.show(Tree(a.n), n + 1)
  inline given tree(using l: => Show[Leaf]): Show[Tree] = new Show[Tree]:
    def show(a: Tree, n: Int): String = if n > 3 then "deep" else "tree:" + l.show(Leaf(a.n), n + 1)
@main def run(): Unit =
  println(summon[Show[Tree]].show(Tree(1), 1))
