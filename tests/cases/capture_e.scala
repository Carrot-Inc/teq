// `catch` with a handler expression binds the exception to a local, in whose scope the handler
// is evaluated.
def `e$0`(): Throwable => Int = _ => 7

@main def main(): Unit =
  println(try throw RuntimeException("x") catch `e$0`())
