package eta

import scala.annotation.targetName

// Members of other target names, exported under other names: each forwarder keeps the name the
// export gives it and carries the member's `@targetName`, by which the class files name it, as
// scalac's copy of the member's annotations has it.
object Impl:
  @targetName("execute") def original(x: Int = 41): Int = x + 1
  @targetName("plus") def +(y: Int): Int = y + 100

object Relay:
  export Impl.{original as renamed, + as add}
