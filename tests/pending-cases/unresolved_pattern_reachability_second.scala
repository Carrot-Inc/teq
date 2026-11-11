// The same, with a second case between: no reachability warning for it or the wildcard after a typer error
// (Phases.scala:354-355). Pending as unresolved_pattern_reachability is.
case class Box(value: Int)
def f(x: Box): Int = x match {
  case Box(Nope.value) => 1
  case Box(2) => 2
  case _ => 0
}
