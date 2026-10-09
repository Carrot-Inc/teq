package pma

trait Parent:
  def run[A](a: A): Any

// A generic refinement member, legal where the parent declares the method.
object PolyMember:
  def keep(f: Parent { def run[A](a: A): A }) = f
