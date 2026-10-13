package nda

// The constructor defaults of a case class nested in a class are getters of its companion, an object
// of each instance (`$lessinit$greater$default$N`, `Desugar.defaultGetters`), which a downstream of either
// compiler reads, the synthesized `apply` taking them (`Applications.findDefaultGetter`).
class O(val n: Int):
  case class K(x: Int = n, y: String = "d")
