// A lambda with no expected type whose parameter is a plain argument of the callee's repeated parameter:
// dotty types it from the callee's parameter type translated from the repeated one, the sequence
// (`Typer.inferredFromTarget`, `translateFromRepeated`; its comment names neg/i14367 for the
// misprediction), and the argument does not fit the element. scalac 3.8.4: `Found: (x : Seq[String])
// Required: String`.
// expect: 9:19: error: type mismatch: found Seq[String], required String
def join(xs: String*): String = xs.mkString("/")

val f = x => join(x)
