package errs

/** A constructor parameter's type, a type parameter's bound and a self type that are not there. */
class Ctor(x: NoCtorType):
  def label: String = "ctor"

class Bounded[T <: NoBound]:
  def label: String = "bounded"

trait Selfish:
  self: NoSelf =>
  def label: String = "selfish"
