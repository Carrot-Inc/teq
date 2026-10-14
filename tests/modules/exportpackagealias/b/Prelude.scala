package pre

object Styles:
  type display = String
  type columns = Int
  type direction = Long
  type position = Double
  type target = Boolean

// The type forwarders' order, Styles' through the wildcard, is no order the names were interned in:
// the whole build meets `target` and `position` first in a's `Words`, b's own build in `Styles`.
trait PreludeBase:
  export lib.{Eq, EqPair}
  export lib.syntax.{Same, SamePair}
  export Styles.*

object Prelude extends PreludeBase
