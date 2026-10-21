package marks
import O.{x as _, *, given Int}
import P.x
object Left { def value = y }
object Right { def value = x + summon[Int] }
