package edits.comments
object O { val x = 1; val y = 2; val z = 3 }
// before
import O.{x /* the x */, y, z} // trailing
/* lead */ import O.y
import O.z // gone
object Main { def run: Int = x }
