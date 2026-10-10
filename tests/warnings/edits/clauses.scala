package edits.clauses
object O { val x = 1 }
object P { val y = 2 }
object Q { val z = 3 }
object R { val a = 4; val b = 5; val c = 6 }
import O.x, P.y
import O.x, P.y, Q.z
import Q.z, R.{a, b, c}
object Main { def run: Int = y + z + a + c }
