// jars: cats-kernel-sjs cats-core-sjs shapeless3-deriving kittens
// teq: --max-inlines 80
//> using dep org.typelevel::kittens:3.5.0
//> using option -Xmax-inlines 80
// kittens' `derives Eq` over a `List` of a class that has no `Eq` of its own: the list derives through its sum, whose
// `::` holds the list again, so the derivation goes through the recursion (`tests/cases/derive_recursive_list_shapes`
// has the libraries' shapes written out). scalac 3.8.4 prints `true`, `false`.
import cats.Eq
import cats.derived.*
object Models:
  final case class Item(id: String)
  final case class Element(items: List[Item]) derives Eq

@main def run(): Unit =
  import Models.*
  println(Eq[Element].eqv(Element(List(Item("a"), Item("b"))), Element(List(Item("a"), Item("b")))))
  println(Eq[Element].eqv(Element(List(Item("a"), Item("b"))), Element(List(Item("a"), Item("c")))))
