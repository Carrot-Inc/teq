// jars: scala-library
// std: lean scala-library
// A concatenation evaluates its operands left to right and then renders them, in one call:
// an effect of a later operand comes before the `toString` of an earlier one. The chain is
// the operands of the `+` at the head, through a block's result; an operand to the right is
// evaluated as it stands, a chain of its own included, and so is a call of `toString`.
class Loud(s: String):
  override def toString: String =
    println("  render " + s)
    s

class Meters(val v: Int) extends AnyVal:
  override def toString: String =
    println("  render meters")
    v.toString + "m"

def effect(s: String): String =
  println("  effect " + s)
  s

def nothing(): Unit = println("  effect unit")

@main def run(): Unit =
  val l = new Loud("x")
  val sb = new java.lang.StringBuilder("a")
  println("" + sb + { sb.append("b"); "" })
  println("1:" + l + effect("y") + new Loud("z"))
  println(("p:" + l) + effect("e"))
  println("a" + ("b" + l) + effect("c"))
  println(({ val h = "h:"; h + l }) + effect("c"))
  println((if l != null then "t:" + l else "f") + effect("c"))
  println("" + l.toString + effect("explicit"))
  println(s"$l${effect("inside")}" + effect("outside"))
  println("" + new Loud("a") + s"${new Loud("b")}${effect("in")}" + effect("out"))
  var n = 1
  println("" + n + { n = 5; "" } + n)
  println("" + new Meters(3) + 'c' + 1 + 2L + 1.5 + 2.5f + true + () + nothing() + null + (null: String) + (3: Byte) + (4: Short) + effect("end"))
  println("" + 1 + 2 + n + 3 + 'c' + effect("numbers"))
  println("tags \u0001 and \u0002 in a literal, " + l + " \u0001" + effect("!") + "\u0002")
  println(("" + l + "\u0001").length)
  try println("" + l + { if n == 5 then throw new RuntimeException("operand"); "" })
  catch case e: RuntimeException => println("  threw " + e.getMessage)
