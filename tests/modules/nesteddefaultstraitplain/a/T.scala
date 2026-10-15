package ndtp

// A plain class nested in a trait with constructor defaults reading the trait's state: `Desugar.classDef`
// gives it a companion for the getters, an object of each instance of the trait, which the classes mixing
// the trait in hold (`T.Item()`), its `$lessinit$greater$default$N` instance methods reading the trait's
// members through the companion's outer; a downstream selects the getter on that companion
// (`Applications.findDefaultGetter`), whichever compiler wrote either module.
trait T {
  val n = 4
  class Item(val x: Int = n)(val y: Int = x + n)
}
class Impl extends T
