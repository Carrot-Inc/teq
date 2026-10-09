package pna

// A polymorphic function type of named parameters, which a call names.
object PolyNamed:
  def keep(f: [A] => (first: A, second: A) => A) = f
