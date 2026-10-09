package dpoa

trait Ctx:
  type T

// A refinement's method whose later clause's parameter type names the first clause's
// parameter, the result naming none.
object ParamOnly:
  def keep(f: AnyRef { def run(c: Ctx)(x: c.T): Int }): Unit = ()
