// A creation in a function's second call, which runs as lowered code: the same proof as the walker's first.
class Wide extends jlib.Span(3)

def make(b: Boolean): Int = if b then new Wide().n else 0

@main def run(): Unit =
  println(make(false))
  println(make(true))
