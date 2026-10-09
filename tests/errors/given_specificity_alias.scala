// expect: 14:47: error: ambiguous given instances for ValueType[Dict[String], Obj]: vtObj, vtDict
// A parameterised alias's parameter is invariant where the alias uses it at two variances, so
// specificity sees no contravariant argument through `Simple[Obj]` and neither given wins (scalac: E172).
final case class ValueType[-A, +U](name: String)
class Obj
class Dict[A] extends Obj
object VT:
  type Simple[A] = ValueType[A, A]
object Aliased:
  implicit val vtObj: VT.Simple[Obj] = ValueType("obj")
  implicit def vtDict[A]: ValueType[Dict[A], Obj] = ValueType("dict")
@main def main(): Unit =
  import Aliased.*
  println(summon[ValueType[Dict[String], Obj]].name)
