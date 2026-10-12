// expect: 11:24: error: type mismatch: found "boom", required C$1
// expect: 1 error found
// The fresh abstract type a GADT pattern leaves (`C$1` in `X = List[C$1]`) takes no
// `String`: scalac's `Found: List[String], Required: X`, reported at the list; here at the
// element, against `List.apply`'s variable, which the message shows as its upper bound
// (`TypeMismatch`'s `reported`).
sealed trait Tree[X]
final case class Node[C](x: C) extends Tree[List[C]]
final case class Leaf(i: Int) extends Tree[Int]
def meth[X](t: Tree[X]): X = t match
  case Node(x) => List("boom")
  case Leaf(i) => i
@main def Main(): Unit = println(meth(Leaf(1)))
