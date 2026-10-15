package ndc

// A case class nested in a class beside its written companion, its defaults in two clauses reading the
// enclosing instance's state and the clause before theirs: each getter is the companion's, taking the
// parameters of the clauses before its own (`Desugar.defaultGetters`), and a creation's every default is
// selected on the companion of the creation's prefix (`Applications.findDefaultGetter`), the receiver and
// the arguments evaluated once, in order.
class Outer(val n: Int) {
  var made = 0
  case class Item(a: Int, b: Int = n + 1)(val c: Int = a + b + n) {
    made += 1
  }
  object Item {
    def tag = "item"
    def first: Item = Item(0)()
  }
}
