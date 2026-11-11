// An unresolved stable identifier in a constructor pattern, then a wildcard: the pattern matcher's reachability would
// call the wildcard unreachable except for null, but it does not run after a typer error (Phases.scala:354-355), as
// space.rs's pattern_matcher_warnings_wait_for_a_program_without_errors test holds. Pending: NotFoundMsg's text (the
// typer's messages) and the stable-identifier error UniqueMessagePositions hides (D2).
case class Box(value: Int)
def f(x: Box): Int = x match {
  case Box(Nope.value) => 1
  case _ => 0
}
