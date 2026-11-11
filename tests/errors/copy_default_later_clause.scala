// A case class's `copy` has defaults for its first parameter clause alone (dotty's Desugar.copyMeths): a later
// clause's parameter has no `copy$default$N`, whatever its access.
// expect: 7:25: error: value copy$default$2 is not a member of C
// expect: 1 error found
case class C(x: Int)(private val y: Int)

def leak(c: C): Int = c.copy$default$2
