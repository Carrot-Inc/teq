package edits.comments
object O { val x = 1; val y = 2; val z = 3 }
// before
import O.x // trailing
/* lead */ 
 // gone
object Main { def run: Int = x }
