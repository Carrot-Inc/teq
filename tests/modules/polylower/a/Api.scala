package plw

// A polymorphic function type whose parameter has a lower bound, `[A >: String]`, which its
// apply keeps (dotty's `Parsers.typeBounds`, `TypeBoundsTree(lower, upper)`).
object Api:
  def keep(f: [A >: String] => (a: A) => A) = f
