package ilb2

import ila2.Macros

// Inline expansions in another file of the build: each expansion is the caller's code (teq
// writes no inline bodies to TASTy yet, so no expansion crosses a module).
object Use:
  val a: Int = Macros.twice(21)
  val b: String = Macros.describe("x")
  val c: Int = Macros.pick(true)
  val d: Int = Macros.total(1, 2, 3)
