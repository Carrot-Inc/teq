// A stable-identifier pattern whose path does not resolve. Pending: scalac's
// "Not found: Nope - did you mean None?" (NotFoundMsg with its did-you-mean,
// the typer's messages' owner), and the stable-identifier error at the same
// place, which scalac's reporter hides (UniqueMessagePositions, D1's item 2).
def f(x: Int): Int = x match { case Nope.value => 1; case _ => 0 }
