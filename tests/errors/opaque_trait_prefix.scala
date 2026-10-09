// expect: 12:24: error: type mismatch: found Html.Tag[Node], required Svg.Tag[Node]
// expect: 13:17: error: type mismatch: found Html.Tag[Node], required String
// The copies of a trait's opaque type that two objects deriving from it have are two types, opaque
// outside the trait (scalac: E007 at both lines).
trait TagKit[Top]:
  final opaque type Tag[+N <: Top] = String
  def apply[N <: Top](name: String): Tag[N] = name
trait Node
object Html extends TagKit[Node]
object Svg extends TagKit[Node]
val h: Html.Tag[Node] = Html[Node]("div")
val s: Svg.Tag[Node] = h
val t: String = Html[Node]("div")
