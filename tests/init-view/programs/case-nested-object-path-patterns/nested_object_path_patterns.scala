// teq: --werror
// Stable paths to objects nested in a class as patterns: `a.R` and `b.R` are two values of one
// class, so the second case is reachable, and a wildcard after them is reachable too.
class O:
  object R
  object S

def which(x: Any, a: O, b: O): String = x match
  case a.R => "a.R"
  case b.R => "b.R"
  case a.S => "a.S"
  case _ => "other"

@main def run(): Unit =
  val a = O()
  val b = O()
  println(which(a.R, a, b))
  println(which(b.R, a, b))
  println(which(a.S, a, b))
  println(which(b.S, a, b))
