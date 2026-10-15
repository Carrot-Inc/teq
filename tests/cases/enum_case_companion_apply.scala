// A class case of an enum beside an object of its name in the enum's companion, with `apply`s of its own
// or none (indigo's `Layer.Content`, `QuadTree.Branch`): the case's own `apply` is among the object's
// alternatives, as `DesugarEnums` makes the case a case class whose companion the object is.
enum Layer:
  case Stack(layers: List[Layer])
  case Content(nodes: List[Int], name: Option[String])
  case Branch(left: Int, right: Int)
object Layer:
  object Content:
    val empty: Layer.Content = Layer.Content(Nil, None)
    def apply(nodes: Int*): Layer.Content = Layer.Content(nodes.toList, None)
  object Branch:
    val unit: Layer = Branch(1, 1)
    def twice(n: Int): Layer = Branch(n, n)

@main def run(): Unit =
  println(Layer.Content(1, 2))
  println(Layer.Content.empty)
  println(Layer.Content(List(3), Some("x")))
  println(Layer.Branch.twice(4))
  println(Layer.Branch(5, 6))
