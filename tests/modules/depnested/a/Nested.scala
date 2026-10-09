package dna

trait Ctx:
  type T
  val t: T

// A dependent function type whose result is one that names the outer parameter.
object Nested:
  def keep(f: (c: Ctx) => (x: c.T) => c.T) = f
