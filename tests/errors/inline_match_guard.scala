// expect: cannot reduce inline match with
// expect: patterns :  case _: Int if b
// A matching case whose guard is no constant stops the reduction, as scalac 3.8.4's
// `InlineReducer.reduceCase` does: `choose(1, b)` is refused, where a constant `false` guard gives
// way to the next case (tests/cases/inline_expansion_rules).
inline def choose(x: Any, b: Boolean): Int =
  inline x match
    case _: Int if b => 1
    case _ => 2
def run(b: Boolean): Int = choose(1, b)
