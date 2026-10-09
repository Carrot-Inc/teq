package use

// The local `x` reads `lib.x`, which its module writes `x` while no other package defines one.
@main def main(): Unit =
  val x = lib.x()
  println(x)
