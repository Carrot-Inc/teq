package dca

trait Ctx:
  type T

// A refinement's dependent method across clauses, in a parameter and the inferred result.
object Cross:
  def keep(f: AnyRef { def run(c: Ctx)(x: c.T): c.T }) = f
