// expect: 16:94: error: no given instance of type Eq[List[Tree]] was found for parameter x
// expect: 1 error found
// As in scalac, a local alias given is not in scope in its own right-hand side: the recursive
// Eq[Tree] that Eq[List[Tree]] needs is not found there.
trait Eq[A]:
  def eqv(a: A, b: A): Boolean
final class EqFn[A](f: (A, A) => Boolean) extends Eq[A]:
  def eqv(a: A, b: A): Boolean = f(a, b)
object Eq:
  def instance[A](f: (A, A) => Boolean): Eq[A] = new EqFn(f)
given Eq[Int] = Eq.instance(_ == _)
given [A](using e: Eq[A]): Eq[List[A]] =
  Eq.instance((x, y) => x.length == y.length && x.zip(y).forall((a, b) => e.eqv(a, b)))
final case class Tree(label: Int, children: List[Tree])
@main def main(): Unit =
  given treeEq: Eq[Tree] = Eq.instance((a, b) => a.label == b.label && summon[Eq[List[Tree]]].eqv(a.children, b.children))
  println(summon[Eq[Tree]].eqv(Tree(1, List(Tree(2, Nil))), Tree(1, List(Tree(2, Nil)))))
