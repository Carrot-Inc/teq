//> using platform js
// A transparent inline method stands where its call stood, to a chain of `+` as well; a plain
// one's call ends the chain.
import Inlines.{cat, plain}

class Loud(s: String):
  override def toString(): String =
    println("  render " + s)
    s

def tail(): String =
  println("  tail")
  "!"

transparent inline def catInferred(inline x: Any) = "" + x
transparent inline def catBound(x: Any): String = "" + x
transparent inline def catBlock(inline x: Any): String = { println("  stat"); "" + x }
transparent inline def catMatch(inline x: Any): String = x match { case s: String => s; case o => "" + o }
transparent inline def catIf(inline x: Any, c: Boolean): String = if c then "" + x else ""
transparent inline def catConstIf(inline x: Any): String = if true then "" + x else ""
transparent inline def catInlineIf(inline x: Any, inline c: Boolean): String = inline if c then "" + x else ""
transparent inline def catInlineMatch(inline x: Any, inline n: Int): String = inline n match { case 1 => "" + x; case _ => "" }
transparent inline def outerT(inline x: Any): String = plain(x)
inline def outerP(inline x: Any): String = cat(x)
transparent inline def outerTT(inline x: Any): String = cat(x)
transparent inline def ascribedT(inline x: Any): String = ("" + x): String
transparent inline def passed(inline s: String): String = s
inline def passedPlain(inline s: String): String = s
transparent inline def tailed(inline x: Any): String = "" + x + tail()

@main def main(): Unit =
  val l = new Loud("x")
  val yes = l != null
  val any: Any = l
  println("transparent"); println(cat(l) + tail())
  println("plain"); println(plain(l) + tail())
  println("transparent, inferred type"); println(catInferred(l) + tail())
  println("transparent, bound parameter"); println(catBound(l) + tail())
  println("transparent, block"); println(catBlock(l) + tail())
  println("transparent, match"); println(catMatch(any) + tail())
  println("transparent, if"); println(catIf(l, yes) + tail())
  println("transparent, if on a constant argument"); println(catIf(l, true) + tail())
  println("transparent, constant if"); println(catConstIf(l) + tail())
  println("transparent, inline if"); println(catInlineIf(l, true) + tail())
  println("transparent, inline match"); println(catInlineMatch(l, 1) + tail())
  println("transparent calling plain"); println(outerT(l) + tail())
  println("plain calling transparent"); println(outerP(l) + tail())
  println("transparent calling transparent"); println(outerTT(l) + tail())
  println("transparent, ascribed body"); println(ascribedT(l) + tail())
  println("transparent passing a chain"); println(passed("" + l) + tail())
  println("plain passing a chain"); println(passedPlain("" + l) + tail())
  println("transparent, ascribed call"); println((cat(l): String) + tail())
  println("transparent, toString of the call"); println(cat(l).toString + tail())
  println("transparent with a tail inside"); println(tailed(l) + tail())
  println("transparent macro"); println(Macros.tmacro(l) + tail())
  println("plain macro"); println(Macros.pmacro(l) + tail())
  println("transparent in a quote"); println(Macros.quotedTransparent(l, tail()))
  println("plain in a quote"); println(Macros.quotedPlain(l, tail()))
  println("ascribed transparent in a quote"); println(Macros.quotedAscribedTransparent(l, tail()))
