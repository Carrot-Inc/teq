package fix.canequal

// The pickled `equals` of a case class as `SyntheticMembers.equalsBody` writes it: the primitive
// fields first, then `that.canEqual(this)`, which a final class keeps where it defines
// `canEqual` itself and drops where the rule's would answer.
object CanEqualCalls:
  var n: Int = 0
final case class CanEqualOwn(x: Int):
  override def canEqual(that: Any): Boolean = { CanEqualCalls.n += 1; false }
final case class CanEqualPlain(s: String, x: Int, b: Boolean)
case class CanEqualOpen(s: String, n: Long)
