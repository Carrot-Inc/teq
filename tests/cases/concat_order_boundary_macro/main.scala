class Loud(s: String):
  override def toString(): String =
    println("  render " + s)
    s

def tail(): String =
  println("  tail")
  "!"

@main def main(): Unit =
  val l = new Loud("x")
  println("a macro's own chains"); println(Macros.built())
  println("quoted toString"); println(Macros.viaToString(l, tail()))
  println("quoted ascription"); println(Macros.viaAscription(l, tail()))
  println("quoted cast"); println(Macros.viaCast(l, tail()))
  println("quoted constant if"); println(Macros.viaConstantIf(l, tail()))
  println("quoted if"); println(Macros.viaIf(l, tail()))
  println("cast of a splice"); println(Macros.castSplice(l, tail()))
  println("plain splice"); println(Macros.plainSplice(l, tail()))
  println("ascribed splice"); println(Macros.ascribedSplice(l, tail()))
  println("inline call in a quote"); println(Macros.inlineCallInQuote(l, tail()))
  println("reflected ascription"); println(Macros.reflected(l, tail()))
  println("reflected constant if"); println(Macros.reflectedIf(l, tail()))
  println("reflected toString"); println(Macros.reflectedToString(l, tail()))
  println("reflected plain"); println(Macros.reflectedPlain(l, tail()))
  println("reflected block"); println(Macros.reflectedBlock(l, tail()))
  println("asExprOf"); println(Macros.asExprOfString(l, tail()))
  println("a tree used with an ascription and without"); println(Macros.twice(l, tail()))
