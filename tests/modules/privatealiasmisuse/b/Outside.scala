package outside

import privatealiasmisuse.*

// The companion's `apply` seen from a subclass of the products' trait: its private alias is no
// member there (`TypeOps.asSeenFrom`, `MissingType`), in either clause.
object Elsewhere extends Scope:
  val cmd = Cmd(1)(List("x"))
