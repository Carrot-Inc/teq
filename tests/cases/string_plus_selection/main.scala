// `+` selected on a `String` is the member scalac gives the class, `+(x$0: Any): String`, and
// makes the concatenation that `s + x` makes: a chain of selected and infix `+` is one chain,
// which scalac's JVM backend evaluates before it renders, and `toString`, an ascription,
// `String.valueOf` and `concat` end it; in a quote and through an inline parameter as well.
class Loud(s: String):
  override def toString(): String =
    println("  render " + s)
    s

def effect(): String =
  println("  effect")
  "!"

type S = String

inline def selected(inline s: String): String = s.+(effect())

@main def main(): Unit =
  val l = new Loud("x")
  println("selected chain"); println("".+(l).+(effect()))
  println("infix then selected"); println(("" + l).+(effect()))
  println("selected then infix"); println("".+(l) + effect())
  println("on a val"); val v: String = "v"; println(v.+(l).+(effect()))
  println("on an alias"); val a: S = "a"; println(a.+(l).+(effect()))
  println("inline parameter"); println(selected("".+(l)))
  println("toString"); println("".+(l).toString.+(effect()))
  println("ascription"); println(("".+(l): String).+(effect()))
  println("valueOf"); println(String.valueOf("".+(l)).+(effect()))
  println("concat"); println("".+(l).concat(effect()))
  println("chain after a boundary"); println(("".+(l): String).+(new Loud("y")).+(effect()))
  println("operands"); println("a".+(1).+('c').+(2L).+(true).+(null).+(1.5))
  println("named"); println("a".+(x$0 = 1))
  println("singletons"); val lit: "a" = "a"; val path: v.type = v; println(lit.+(1).+(path.+(2)))
  println("quoted chain"); println(Macros.quotedChain(l, effect()))
  println("quoted toString"); println(Macros.quotedToString(l, effect()))
  println("spliced chain"); println(Macros.splicedChain(l, effect()))
  println("reflected"); println(Macros.reflected(l, effect()))
