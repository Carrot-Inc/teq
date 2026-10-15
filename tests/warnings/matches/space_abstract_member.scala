// A scrutinee of an abstract type member or a type parameter: a literal's type and a class are
// tested against its upper bound (dotty's `provablyDisjoint` of the member's `superType`,
// Space.scala 747), so an unbounded `p.Out` keeps `Boolean`'s cases and a bounded one drops
// what its bound excludes.
trait Property:
  type Out
  val valueOf: Out
trait IntProperty:
  type Out <: Int
  val valueOf: Out
trait FlagProperty:
  type Out <: Boolean
  val valueOf: Out
object Render:
  def render(p: Property): String = p.valueOf match
    case x: Boolean => x.toString
    case x: String => x
  def literal(p: Property): String = p.valueOf match
    case true => "yes"
    case false => "no"
    case _ => "other"
  def bounded(p: IntProperty): String = p.valueOf match
    case x: String => x
    case n: Int => n.toString
  def flags(p: FlagProperty): String = p.valueOf match
    case true => "yes"
    case false => "no"
  def param[T](t: T): String = t match
    case true => "yes"
    case x: String => x
    case _ => "other"
  def boundedParam[T <: Int](t: T): String = t match
    case x: String => x
    case n: Int => n.toString
