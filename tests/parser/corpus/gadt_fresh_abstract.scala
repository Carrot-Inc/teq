// A pattern whose class fixes the scrutinee's type parameter through a parameter of its own
// (`Node[C] extends Tree[List[C]]`) makes that parameter a fresh abstract type in the case:
// `X` is `List[C$1]` there, so the field's list is an `X`.
sealed trait Tree[X]
final case class Node[C](x: C) extends Tree[List[C]]
final case class Leaf(i: Int) extends Tree[Int]

def meth[X](t: Tree[X]): X = t match
  case Node(x) => List(x)
  case Leaf(i) => i

@main def Main(): Unit =
  println(meth(Leaf(1)))
  println(meth(Node("a")))
