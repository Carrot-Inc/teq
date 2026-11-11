// jars: cats-kernel-sjs cats-core-sjs shapeless3-deriving kittens
// teq: --max-inlines 80
//> using dep org.typelevel::kittens:3.5.0
//> using option -Xmax-inlines 80
// kittens' `derives Eq` over an enum that holds itself and has no `Eq` of its own (`Node(l: Tree, r: Tree)`): as
// `derive_recursive_list`, through the enum's own sum. scalac 3.8.4 prints `true`, `false`.
import cats.Eq
import cats.derived.*
import scala.deriving.Mirror
object Models:
  enum Tree:
    case Leaf
    case Node(l: Tree, r: Tree)
  final case class Element(t: Tree) derives Eq

@main def run(): Unit =
  import Models.*, Tree.*
  println(Eq[Element].eqv(Element(Node(Leaf, Node(Leaf, Leaf))), Element(Node(Leaf, Node(Leaf, Leaf)))))
  println(Eq[Element].eqv(Element(Node(Leaf, Node(Leaf, Leaf))), Element(Node(Node(Leaf, Leaf), Leaf))))
