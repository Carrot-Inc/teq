// A static field read in lowered code is placed at the read, as the walker places it.
def hits(b: Boolean): Int =
  if b then jlib.Registry.hits
  else 0

@main def run(): Unit =
  println(hits(false))
  println(hits(true))
