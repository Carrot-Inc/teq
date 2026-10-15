package ndn

// A case class nested in a class beside a written companion whose `apply` overloads the synthesized one
// by a parameter of the same name, which a named argument resolves (`Item(x = "abcd")`), and one whose
// `apply` has the synthesized one's signature and so replaces it (`Desugar.classDef` leaves it out):
// `o.Exact(2)` is the written one's, a creation with `new` the constructor's default getter's on the
// companion (`Applications.findDefaultGetter`).
class Outer(val n: Int) {
  case class Item(x: Int = n)
  object Item {
    def apply(x: String): Item = new Item(x.length)
  }
  case class Exact(x: Int = n)
  object Exact {
    def apply(x: Int): Exact = new Exact(x + 100)
  }
}
