package edits.braces_last
object O { val x = 1; val y = 2; val z = 3 }
import O.{x, y, z}   
object Main { def run: Int = x + y }
