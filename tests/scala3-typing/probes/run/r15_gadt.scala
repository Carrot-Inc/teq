sealed trait Tree[+A]
final case class Leaf[+B](b: B) extends Tree[B]
final case class Node[+C](l: List[C]) extends Tree[List[C]]
object Test:
  def meth[X](tree: Tree[X]): X = tree match
    case Leaf(v) => v
    case Node(x) => List("boom")
@main def run(): Unit =
  val res: List[Int] = Test.meth(Node(List(42)))
  println(res.head + 1)
