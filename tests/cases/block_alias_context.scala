// A block's alias completed in its own context, whichever statement names it first: the block's
// frames and the imports before its definition, never a nested block's (`Namer.indexExpanded`).
object A { type F = Int }
object B { type F = String }
@main def run(): Unit =
  import A.F
  // `R` named first inside a nested block whose import shadows `F`: completed in its own context.
  def g: Int = { import B.F; val r: R = 1; r }
  type R = F
  println(g)
  run2()
def run2(): Unit =
  // `S` named before the import its definition follows: that import is in its context.
  def h: S = 2
  import A.F
  type S = F
  println(h)
