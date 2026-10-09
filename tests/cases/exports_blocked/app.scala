package util

import util.Prelude.*

// `HasHelper` is only reachable through the prelude, which in turn exports this object.
object DeskAppUtils extends HasHelper:
  def own(x: Int): Int = helper(x) * 2

object Prelude extends sharedlib.PreludeCore:
  export util.DeskAppUtils.*
