//> using platform js
// The chain scalac's backend sees after inlining: an inline parameter's argument joins the chain
// it is spliced into, an inline method's call is an operand of its own.
class Loud(s: String):
  override def toString: String =
    println("  render " + s)
    s

def effect(s: String): String =
  println("  effect " + s)
  s

inline def inside(a: Any, b: String): String = "in:" + a + b

inline def stable(a: Any): String = "in:" + a

inline def cat(inline l: String, inline r: String): String = l + ":" + "." + "," + ";" + r

inline def twice(inline l: String, inline r: String): String = l + ":" + "." + "," + ";" + r

@main def main(): Unit =
  val loud = new Loud("x")
  println("a chain in an inline method")
  println(inside(loud, effect("b")))
  println("a chain an inline parameter supplies, one site")
  println(cat("" + loud, effect("!")))
  println("two sites")
  println(twice("" + loud, effect("1")))
  println(twice("" + loud, effect("2")))
  println("an inline method with bound parameters as the head")
  println(inside(loud, effect("b")) + effect("c"))
  println("an inline method whose argument is stable as the head")
  println(stable(loud) + effect("c"))
  println("an inline method with inline parameters as the head")
  println(cat("" + loud, effect("!")) + effect("c"))
  println("an inline method's call as an inline parameter's argument")
  println(cat(stable(loud), effect("!")))
