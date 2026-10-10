package edits.givens
object O { given Int = 1; given String = "s"; val v = 2; val w = 3 }
object G { given Double = 1.0 }
import O.{v, w, given Int, given String}
import G.given
object Main { def run: Int = summon[Int] + v }
