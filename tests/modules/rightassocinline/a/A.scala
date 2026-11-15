// Right-associative operators of another module: the argument of an `inline` parameter is not bound ahead of the
// application (`LiftImpure.liftArgs` leaves it; it is evaluated where the body reads it, twice here, after the
// receiver), and neither is a by-name parameter's. A loaded method's signature does not say which of its parameters
// are inline; its inline body does.
package probe
class InlineBox:
  inline def +:(inline x: Int): Int = x + x
class ByName:
  def +:(x: => Int): Int = { print("m"); x + x }
