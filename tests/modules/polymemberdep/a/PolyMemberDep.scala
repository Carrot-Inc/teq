package pmda

trait Parent:
  def run[A](a: A): Any

// A generic refinement member whose result names its term parameter.
object PolyMemberDep:
  def keep(f: Parent { def run[A](a: A): a.type }) = f
