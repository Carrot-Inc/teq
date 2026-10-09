package pda

// A polymorphic function type whose result names its term parameter.
object PolyDep:
  def keep(f: [A] => (a: A) => a.type) = f
  val f: [A] => (a: A) => a.type = [A] => (a: A) => a
