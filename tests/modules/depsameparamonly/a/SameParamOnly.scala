package dspa

trait Ctx:
  type T

// A refinement's method whose second parameter's type names the first, in one clause, the
// result naming neither.
object SameParamOnly:
  def keep(f: AnyRef { def run(c: Ctx, x: c.T): Int }): Unit = ()
