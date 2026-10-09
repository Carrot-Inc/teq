// The fresh abstract types a GADT pattern makes keep their parameters' declared bounds over
// each other: `A$1 >: String` and `B$1 <: A$1` for a `Node[A >: String, B <: A]`.
sealed trait Tree[X]
final case class Node[A >: String, B <: A](a: A, b: B) extends Tree[List[A]]

def meth[X](t: Tree[X]): X = t match
  case Node(a, b) => List(b, "s", a)

@main def Main(): Unit = println(meth(Node[String, String]("x", "y")))
