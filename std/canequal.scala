package scala

// Equality between unrelated types is not checked; the type exists so that `derives CanEqual`
// and givens of it compile.
trait CanEqual[-L, -R]

object CanEqual:
  private final class Derived extends CanEqual[Any, Any]
  private val instance: CanEqual[Any, Any] = new Derived
  def derived: CanEqual[Any, Any] = instance
