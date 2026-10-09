// The first alternative's expansion of the argument `M.arg`, ahead of the later phase where the
// transparent extension copies it, reports an error that is held for the unit's flush; the
// alternative's retraction puts the call back pending and takes the error with it, and the second
// alternative's expansion reports none: `i1`, as scalac.
trait Ops:
  extension [T](x: Int)
    transparent inline def combine(inline y: String): String = "s" + y
  extension [T](x: Int)
    transparent inline def combine(inline y: Int): String = { M.tick; "i" + y }
object O extends Ops
@main def run(): Unit = println(O.combine[Unit](1)(M.arg))
