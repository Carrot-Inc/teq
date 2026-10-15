package ndpa

// A case class nested in a class beside a written companion whose initialiser is its parents': a class
// parent's argument (`Parent(init())`) and a trait's body. `o.Item(..)` selects the companion's `apply`
// on the companion the prefix gives, saved once (`EtaExpansion.liftApp`), so the receiver runs first,
// then the companion's initialiser, the argument and the default getter selected on that same companion
// (`Applications.findDefaultGetter`).
class Parent(x: Int)
trait Init {
  println("trait companion")
}
class Outer {
  var state = 0
  def init(): Int = { println("companion"); state = 10; state }
  case class Item(a: Int, b: Int = { println("default"); state })
  object Item extends Parent(init())
  case class Mixed(a: Int, b: Int = { println("default"); state + 1 })
  object Mixed extends Init
}
