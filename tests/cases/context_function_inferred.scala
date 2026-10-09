// A value of a context function type where no context function is expected of it is applied
// to the given in scope (dotty's `Typer.adaptNoArgsOther`): an inferred definition takes the
// result, computed once where it is defined; one that declares the context function type keeps
// the function, and so does a context function literal (`isContextualClosure`), where an `Any`
// is expected of it too.
val f: Int => (String ?=> Int) = n => (s: String) ?=> { println("applied to " + s); n + s.length }

val anyFunction: Any = (s: String) ?=> s.length

@main def run(): Unit =
  println(anyFunction != null)
  given String = "abc"
  val a = f(1)
  val i: Int = a
  println(i + a)
  val e: String ?=> Int = f(1)
  println(e(using "x"))
  println(e)
  val literal = (s: String) ?=> s.length * 10
  println(literal(using "xy"))
  println(List(f(2)))
