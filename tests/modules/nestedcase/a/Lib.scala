package nca

// A case class nested in a class: its companion is an object of each enclosing instance, made with
// it (`LazyVals.transformSyntheticModule`), holding the factories over that instance
// (`Desugar.classDef`, `ExplicitOuter`), which a module scalac compiles reaches through `o.K()`.
class O(val n: Int):
  case class K(x: Int, y: String):
    def sum = n + x
  def make(i: Int): K = K(i, "m")
