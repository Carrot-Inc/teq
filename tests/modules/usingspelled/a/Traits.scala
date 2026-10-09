package usa

// Using parameters written `using$1` and `using$2`, which keep their spelling in the accessors (`using$1`,
// `usa$Hidden$$using$2`) and in the pickle where only a parameter the parser named (an anonymous one) is scalac's
// `x$N`; scalac's downstream over teq's products implements the same accessors (they were once
// renamed `x$1`).
trait Spelled(val base: Int)(using val `using$1`: Int):
  def answer: Int = base + `using$1`

trait Hidden(val tag: String)(using `using$2`: String):
  def shown: String = tag + `using$2`
