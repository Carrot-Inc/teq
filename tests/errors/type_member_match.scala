// A match on a value of an abstract type is checked through the member's bound.
// expect: 12:3: warning: match may not be exhaustive; missing: Green
// expect: 17:30: error: type mismatch: found Int, required String
// expect: 1 error found
sealed trait Light
case object Red extends Light
case object Green extends Light
trait Signal:
  type Colour <: Light
  def current: Colour
def describe(s: Signal): String =
  s.current match
    case Red => "stop"
def name(s: Signal): String = s.current match
  case Red => "stop"
  case Green => "go"
def bad(s: Signal): String = 1
