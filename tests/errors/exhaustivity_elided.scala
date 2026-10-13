// teq: --wconf any:error
// An exhaustivity warning is `PatternMatchExhaustivity`'s text (messages.scala 909–918): "match may not be
// exhaustive.", a blank line, "It would fail on pattern case: " with six uncovered cases at most, then "(More
// unmatched cases are elided)" on a line of its own where there are more, and nothing where there are not.
// exhaustivity_elided.check holds the whole output, each message's lines as scalac 3.8.4 writes them.
enum E { case A, B, C, D, E, F, G, H, I }
def f(e: E): Int = e match { case E.A => 1 }
enum S { case A, B, C, D, E, F, G }
def g(s: S): Int = s match { case S.A => 1 }
enum R { case A, B, C, D, E, F, G, H }
def h(r: R): Int = r match { case R.A => 1 }
case class N(a: Boolean, b: Boolean, c: Boolean)
def n(x: (N, Boolean)): Int = x match { case (N(true, true, true), true) => 1 }
def m(x: (N, Boolean)): Int = x match
  case (N(true, _, _), _) => 1
  case (_, false) => 2
// expect: exhaustivity_elided.scala:7:20: error: match may not be exhaustive.
// expect: It would fail on pattern case: B, C, D, E, F, G
// expect: (More unmatched cases are elided)
// absent: missing:
// expect: 5 errors found
