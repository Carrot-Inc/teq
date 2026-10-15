package ndr

// A case class nested in a class beside a written companion with a body: `o.Item(..)` is the companion's
// `apply` called on `o.Item`, so the companion's initialiser runs after the receiver and before the
// arguments (`EtaExpansion` lifting a receiver that is not idempotent, `Lifter.liftPrefix`), and the
// default reads the state the initialiser set (`Applications.findDefaultGetter` on that companion).
class Outer(val n: Int) {
  var state = 0
  case class Item(a: Int, b: Int = { println("default"); n + state })
  object Item {
    println("companion")
    state = 10
  }
}
