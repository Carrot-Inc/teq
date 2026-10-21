package edits.aliases
object O { val x = 1; val y = 2; val z = 3 }
import O.{x as a, z as c}
object Main { def run: Int = a + c }
