package ppa

// A polymorphic function type of unnamed parameters, `x$1` and `x$2` in its `apply`.
object PolyPlain:
  def keep(f: [A] => (A, A) => A) = f
