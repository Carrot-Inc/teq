package edits.braces_two_kept
object O { val x = 1; val y = 2; val z = 3; val w = 4 }
import O.{x, y, z, w}
object Main { def run: Int = x + z }
