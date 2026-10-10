// A plain call with arguments among a transparent method's arguments stays pending in each copy the
// expansion makes, with its own receiver and arguments (`Inlining.InliningTreeMap.transform`):
// `M.arg(0)`'s macro runs after `M.tick`'s, at the copy the later phase expands, as in scalac.
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
@main def run(): Unit = println(O.combine[Unit](1)(M.arg(0)))
