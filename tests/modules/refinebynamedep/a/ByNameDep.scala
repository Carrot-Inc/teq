package rbda

trait Ctx:
  type T

// A by-name parameter whose type names the first clause's parameter.
object ByNameDep:
  def keep(f: AnyRef { def run(c: Ctx)(x: => c.T): c.T }) = f
