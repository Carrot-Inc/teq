package uoa

import scala.annotation.unchecked.uncheckedOverride

// A member marked by `@uncheckedOverride` (`RefChecks.isMarkedOverride`), read from this module's products by a class of
// the next one that mixes it in after the concrete one it overrides; the mark read so beside an annotation of another
// class of that name (A2).
trait X { def f: Int }
trait A extends X { @uncheckedOverride def f: Int = 1 }
trait B extends X { def f: Int = 2 }
trait A2 extends X { @uoafake.uncheckedOverride @uncheckedOverride def f: Int = 3 }
trait B2 extends X { def f: Int = 4 }
