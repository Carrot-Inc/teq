package spa

// A case class's selectors `_1` .. `_N` are synthetic definitions of its pickle (dotty's `Desugar.productElemMeths`),
// which a downstream module loads as members as written (`TreeUnpickler.readNewDef`), one implementing a parent's
// abstract `_1`: the previous head dropped them with the members teq supplies itself ("Nest$Box has no member _1").
trait Has[A] { def _1: A }
object Nest:
  case class Box[A](value: A) extends Has[A]
  case class Pair(a: Int, b: String)
