package ndo

// A case class nested in a class beside its written companion that overloads `apply`: the synthesized
// `apply` the overload resolves to constructs through the companion's prefix as the lone one does
// (`Desugar.classDef`), its default the constructor's getter on that companion
// (`Applications.findDefaultGetter`), whichever compiler wrote either module.
class Outer(val n: Int) {
  case class Item(x: Int = n)
  object Item {
    def apply(s: String): Item = new Item(s.length)
  }
}
