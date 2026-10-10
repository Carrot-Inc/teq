package edits.aliases
object O { val x = 1; val y = 2; val z = 3 }
import O.{x as a, y as b, z as c}
import O.x as xx
object Main { def run: Int = a + c }
