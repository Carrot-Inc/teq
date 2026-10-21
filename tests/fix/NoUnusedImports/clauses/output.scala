package edits.clauses
object O { val x = 1 }
object P { val y = 2 }
object Q { val z = 3 }
object R { val a = 4; val b = 5; val c = 6 }
import P.y
import Q.z, R.{a, c}
object Main { def run: Int = y + z + a + c }
