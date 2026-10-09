package dsca

trait Ctx:
  type T

// A refinement's dependent method within one clause, in a parameter and the inferred result.
object SameClause:
  def keep(f: AnyRef { def run(c: Ctx, x: c.T): c.T }) = f
