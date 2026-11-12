// A callee's type argument that names the lambda's parameter, as a singleton and inside a match
// type: dotty reads no callee where the function part holds a parameter's name anywhere
// (`Typer.typedFunctionValue`'s `containsParamRef`), so `x.type` is the parameter's own, typed
// with the body, and not the `x` around the lambda.
def on[A, B](f: A => B): A => B = f
def one[T](x: T): String = x.toString

@main def run(): Unit =
  val x = "outer"
  val f = on(x => one[x.type](x))
  val g = on(x => one[x.type match { case Any => x.type }](x))
  println(f(42) + " " + g(7))
