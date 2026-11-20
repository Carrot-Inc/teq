// Span(int) stores its argument: a Java body the interpreter has none of.
class Wide extends jlib.Span(3)

@main def run(): Unit =
  println("before")
  println(new Wide().n)
