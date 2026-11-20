// A refusal raised under a function value's call in lowered code is placed at the call, as the walker places it.
class Lamp extends jlib.Quiet

def say(b: Boolean): String =
  val f: () => String = new Lamp().hum
  if b then f() else "quiet"

@main def run(): Unit =
  println(say(false))
  println(say(true))
