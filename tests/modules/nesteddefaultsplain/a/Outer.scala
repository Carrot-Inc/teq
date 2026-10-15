package ndp

// A plain class nested in a class with constructor defaults has a companion for their getters, an object
// of each instance (`Desugar.classDef` gives a class whose constructor has defaults `companionDefs` of
// the getters), `Outer$K$` reached through `Outer.K()`, whose `$lessinit$greater$default$N` read the
// enclosing instance's state through the companion's own; a written companion holds them beside its
// own members (`Outer$W$`).
class Outer(val n: Int) {
  class K(val x: Int = n, val y: String = "d")(val z: Int = x * 2)
  class W(val w: Int = n + 1)
  object W {
    def tag = "w"
  }
}
