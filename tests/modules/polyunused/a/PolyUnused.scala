package pua

trait Parent:
  def run[A](a: Int): Any

// A generic refinement member whose type parameter no parameter names: its arity is the
// signature's alone.
object PolyUnused:
  def keep(f: Parent { def run[A](a: Int): Int }) = f
