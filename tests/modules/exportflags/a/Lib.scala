package xflags

// The review's probe of the export representation: a class's export through its value and a
// file's export through a top-level val. Their forwarders are members as scalac makes them, which
// a macro reads in either compiler's pickles: `Exported | Final | Method`, a stable val's
// `StableRealizable` besides, and a type's an `Exported | Final` alias.
class Impl:
  type T = Int
  val n = 7
  def f(x: Int) = x + n
val impl = new Impl
export impl.{T, n, f}
class Wrap(val i: Impl):
  export i.{T, n, f}
