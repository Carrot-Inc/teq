// An unresolved stable identifier nested in a constructor pattern: Box(_)
// is to be no missing case (dotty's ErrorType covers).
// Pending: scalac's "Not found: Nope - did you mean None?" (NotFoundMsg,
// the typer's messages' owner), and the stable-identifier error at the
// same place, which scalac's reporter hides (D2's UniqueMessagePositions).
case class Box(value: Int)
def f(x: Box): Int = x match {
  case Box(Nope.value) => 1
}
