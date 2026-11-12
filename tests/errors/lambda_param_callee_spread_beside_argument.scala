// A lambda under an open formal whose parameters are a plain argument and a sequence argument of the
// callee's one repeated parameter. scalac 3.8.4 refuses the application at both: `Found: (a :
// Seq[String]) Required: String` at `a` (dotty's `inferredFromTarget` reads the repeated parameter's
// type for the argument in its place and none for one past the parameters), and "Sequence argument
// type annotation `*` cannot be used here: it is not the only argument to be passed to the
// corresponding repeated parameter String*" at `b*`. Here no parameter is typed from the callee's
// repeated parameter in such an application, and the lambda is refused at `a`.
// expect: 12:28: error: type mismatch: found Any, required Seq[String] | Array[? <: String]
def on2[B, C, R](f: (B, C) => R): (B, C) => R = f
def join(xs: String*): String = xs.mkString("/")

val f = on2((a, b) => join(a, b*))
