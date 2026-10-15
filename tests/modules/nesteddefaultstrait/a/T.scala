package ndt

// Case classes nested in a trait with constructor defaults reading the trait's state: their getters are
// members of the companion, an object of each instance (`Desugar.classDef`), the written one and the one
// made for a case class without one alike, which the classes mixing the trait in hold (`T.Item()`), and
// a downstream selects the getter on that companion (`Applications.findDefaultGetter`).
trait Trait {
  val n: Int = 4
  case class Item(x: Int = n)
  object Item
  case class Made(y: Int = n + 1)(val z: Int = y * 2)
}
class Impl extends Trait
