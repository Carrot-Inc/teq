package edits.clauses_last_lost
object O { val x = 1 }
object P { val y = 2 }
object Q { val z = 3 }
import O.x, P.y
object Main { def run: Int = x + y }
