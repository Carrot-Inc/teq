// `String.valueOf` and `concat` are calls: their argument is rendered where the call stands, on
// every target.
class Loud(s: String):
  override def toString: String =
    println("  render " + s)
    s

def effect(s: String): String =
  println("  effect " + s)
  s

@main def main(): Unit =
  val l = new Loud("x")
  println(String.valueOf(l) + effect("valueOf"))
  println("c".concat(l.toString) + effect("concat"))
  println("" + String.valueOf(l) + effect("operand") + l)
