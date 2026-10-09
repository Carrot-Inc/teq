class Loud(s: String):
  override def toString(): String =
    println("  render " + s)
    s

def effect(s: String): String =
  println("  effect " + s)
  s

@main def main(): Unit =
  val loud = new Loud("x")
  println("a chain a quote builds")
  println(Macros.joined(loud, effect("b")))
  println("a chain a fold over quotes builds")
  println(Macros.folded(loud, effect("1"), new Loud("y"), effect("2")))
  println("a macro's call as the head")
  println(Macros.folded(loud, effect("1")) + effect("2"))
