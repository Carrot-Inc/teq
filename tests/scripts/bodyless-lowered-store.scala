// A static field written in lowered code is placed at the write, as the walker places it.
def tally(b: Boolean): Int =
  if b then jlib.Registry.hits = 9
  0

@main def run(): Unit =
  println(tally(false))
  println(tally(true))
