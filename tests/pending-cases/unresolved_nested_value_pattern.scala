// An unresolved stable identifier nested in a constructor pattern: its space is its error type's, which covers as
// dotty's ErrorType does, so Box(_) is no missing case (held by space.rs's an_unresolved_selection_pattern_covers test).
// Pending: teq's "not found: Nope" against scalac's "Not found: Nope - did you mean None?" (NotFoundMsg, the typer's
// messages' owner), and teq's stable-identifier error at the same place, which scalac's reporter hides (D2's
// UniqueMessagePositions).
case class Box(value: Int)
def f(x: Box): Int = x match {
  case Box(Nope.value) => 1
}
