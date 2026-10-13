// A top-level `def` of the empty package is its file's `$package` object's member, which the empty
// package's exclusion leaves in; a top-level object there is excluded (J).
def F(x: String): String = x
object J { def apply(x: String): String = x }
object R:
  class F(x: Int)
  val f = F(1)
  class J(x: Int)
  val j = J(1)
object Imp:
  object src:
    def G(x: String) = x
  import src.*
  class H:
    class G(x: Int)
    val g = G(1)
// expect: b.scala:7:11: error: Reference to constructor proxy for class F in object R
// expect: shadows outer reference to method F
// expect: b.scala:16:13: error: Reference to constructor proxy for class G in class H
// expect: shadows outer reference to method G in object src
