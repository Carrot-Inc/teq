// expect: 11:12: error: Stable identifier required, but Hue.valueOf found
// expect: 14:12: error: Stable identifier required, but Hue.fromOrdinal found
// expect: 17:12: error: Stable identifier required, but Ops.twice found
// A method named alone in a pattern's place is no stable identifier: an error (E135), an enum companion's
// lookup as any method.
enum Hue:
  case Red, Blue
object Ops:
  def twice(x: Int): Int = x * 2
def lookup(x: Any): String = x match
  case Hue.valueOf => "matched"
  case _ => "no"
def ordinal(x: Any): String = x match
  case Hue.fromOrdinal => "matched"
  case _ => "no"
def method(x: Any): String = x match
  case Ops.twice => "matched"
  case _ => "no"
