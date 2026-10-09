package mla

// An object the expansions never reach: its initialiser runs when the program first reads it,
// never at a downstream build's expansion of the interpolator.
object Audit:
  println("audit initialised")
  def count: Int = 1
