package ndd

// A case class nested in a trait beside its written companion with a body, the trait reached twice in
// the linearisation of the class mixing it in: the one companion of each instance holds the getters
// (`Desugar.classDef`), its initialiser run once, at the first selection of `apply` or of a getter
// (`Applications.findDefaultGetter`).
trait T {
  val n = 4
  case class Item(x: Int = n)
  object Item {
    println("companion")
  }
}
trait U extends T
class Base extends T
class Impl extends Base with U
