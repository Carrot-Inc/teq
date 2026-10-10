package edits.hiding
object O { val x = 1; val y = 2 }
object P { val x = 3; val q = 4 }
import O.x as _
import P.*
object Main { def run: Int = x }
