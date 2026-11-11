// The names zinc gets for inline accessors (tests/analysis-bytes.sh): C.inc is scalac 3.8.4's
// -Ydump-sbt-inc -Yforce-sbt-phases dump of this file, whose used names are the accessors' names as
// dotty's `zincMangledName` gives them, mangled: an extensible class's `p$C$$inline$x`
// (`InlineAccessorName(ExpandedName(p$C, x))`), a final class's and an object's `inline$y`.
package p

class C:
  private var x = 0
  private def twice(n: Int): Int = n * 2
  inline def get: Int = twice(x)
  inline def set(v: Int): Unit = x = v

final class F:
  private val y = 1
  inline def read: Int = y

object O:
  private val z = 2
  inline def read: Int = z
