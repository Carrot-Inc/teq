package dtfa

trait Ctx:
  type T
  val t: T

// A dependent function type as an alias, an annotated val and an inferred one, whose type
// scalac pickles as a type, the method type's parameter a `PARAMtype`; and inherited.
object Types:
  type F = (c: Ctx) => c.T
  val explicit: (c: Ctx) => c.T = c => c.t
  val inferred = explicit

trait Base:
  def value: (c: Ctx) => c.T

class Derived extends Base:
  def value = Types.inferred
  val inherited = superValue
  def superValue = value
