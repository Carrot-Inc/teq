class Tree
class Term extends Tree
class Inlined(val call: Option[Tree], val bindings: List[Int], val body: Term) extends Term
object Inlined:
  def unapply(x: Inlined): (Option[Tree], List[Int], Term) = (x.call, x.bindings, x.body)
class Lit(val n: Int) extends Term
def f(t: Term): String = t match
  case Inlined(Some(c: Term), _, _) => "call"
  case Inlined(_, _, inner) => "inner " + f(inner)
  case other => "other"
@main def run(): Unit =
  println(f(new Inlined(Some(new Lit(1)), Nil, new Lit(2))))
  println(f(new Inlined(Some(new Tree), Nil, new Lit(2))))
  println(f(new Inlined(None, Nil, new Lit(2))))
  println(f(new Lit(3)))
