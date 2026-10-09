package dia

trait Ctx:
  type T
  val t: T

trait Base:
  def value: (c: Ctx) => c.T

// An inherited member's dependent function type, inferred.
abstract class Derived extends Base:
  val inferred = value
