package upa

// An applied alias's body over its arguments is erased as the emitter reads it: `AC[U1 | U2]` is `(U1 | U2) & Z`, whose
// `Pure` side's class symbol reads the join of `U1 | U2` (TypeErasure.scala:801-804), so `applied` takes `Z`. The
// typer keeps that join for the emitter from this module's own types, so the descriptor doesn't depend on module b.
import scala.caps.Pure
trait Z
trait U1 extends Any with Pure
trait U2 extends Any with Pure
class ZU1 extends Z with U1
type AC[X] = X & Z
type UU = U1 | U2
object L:
  def applied(x: AC[U1 | U2]): String = "applied"
  def aliased(x: AC[UU]): String = "aliased"
