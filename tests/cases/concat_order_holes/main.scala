//> using platform js
// The ends of a chain of `+` and the branches of folded conditions around the holes of a quote,
// around inline parameters and in the trees a macro builds by reflection.
import Macros.*

class Loud(s: String):
  override def toString(): String =
    println("  render " + s)
    s

def tail(): String =
  println("  tail")
  "!"

inline def ascribedParam(inline s: String): String = (s: String) + tail()
inline def toStringParam(inline s: String): String = s.toString + tail()
inline def castParam(inline s: String): String = s.asInstanceOf[String] + tail()
inline def uncheckedParam(inline s: String): String = (s: @unchecked) + tail()
inline def blockParam(inline s: String): String = ({ println("  stat"); s }) + tail()
inline def ascribedBlockParam(inline s: String): String = (({ println("  stat"); s }): String) + tail()
inline def ifParam(inline s: String): String = (if true then s else "") + tail()
inline def ifElseParam(inline s: String): String = (if false then "" else s) + tail()
inline def ifBothParams(inline s: String, inline z: String): String = (if true then s else z) + tail()
inline def ifCondParam(inline c: Boolean, inline s: String): String = (if c then s else "") + tail()

inline def ascribedInline(inline c: Boolean, inline s: String): String = (if (c: Boolean) then s else "") + tail()

@main def main(): Unit =
  val l = new Loud("x")
  val no = l == null
  val yes = l != null
  println("a hole"); println(plainHole("" + l, tail()))
  println("an ascribed hole"); println(ascribedHole("" + l, tail()))
  println("toString of a hole"); println(toStringHole("" + l, tail()))
  println("a cast hole"); println(castHole("" + l, tail()))
  println("an annotated hole"); println(uncheckedHole("" + l, tail()))
  println("a block ending in a hole"); println(blockHole("" + l, tail()))
  println("an ascribed block ending in a hole"); println(ascribedBlockHole("" + l, tail()))
  println("a cast block ending in a hole"); println(castBlockHole("" + l, tail()))
  println("a constant if with a hole in its first branch"); println(ifThenHole("" + l, tail()))
  println("a constant if with a hole in its second branch"); println(ifElseHole("" + l, tail()))
  println("a constant if with holes in both"); println(ifBothHoles("" + l, "" + new Loud("z"), tail()))
  println("an if on a hole"); println(ifCondHole(yes, "" + l, tail()))
  println("an if on a hole filled with a constant"); println(ifCondHole(true, "" + l, tail()))
  println("a constant if around an ascribed hole"); println(ifAscribedHole("" + l, tail()))
  println("an ascribed constant if around a hole"); println(ascribedIfHole("" + l, tail()))
  println("a branch used again"); println(reuse(no, l, tail()))
  println("a branch used again, taken"); println(reuse(yes, l, tail()))
  println("a condition built from a quote"); println(reflected("" + l, tail()))
  println("a constant if used twice"); println(wholeIfTwice("" + l, tail()))
  println("a constant if as a branch"); println(ifAsBranch(yes, "" + l, tail()))
  println("a block over an ascription"); println(blockOverTyped("" + l, tail()))
  println("an ascription over an ascription"); println(typedOverTyped("" + l, tail()))
  println("a constant if over an ascription"); println(ifOverTyped("" + l, tail()))
  println("an ascription over a constant if"); println(typedOverIf("" + l, tail()))
  println("a block over a constant if"); println(blockOverIf("" + l, tail()))
  println("a constant if over a block"); println(ifOverBlock("" + l, tail()))
  println("a quoted constant if spliced"); println(ifOverSplice("" + l, tail()))
  println("an ascribed inline parameter"); println(ascribedParam("" + l))
  println("toString of an inline parameter"); println(toStringParam("" + l))
  println("a cast inline parameter"); println(castParam("" + l))
  println("an annotated inline parameter"); println(uncheckedParam("" + l))
  println("a block ending in an inline parameter"); println(blockParam("" + l))
  println("an ascribed block ending in an inline parameter"); println(ascribedBlockParam("" + l))
  println("a constant if with an inline parameter in its first branch"); println(ifParam("" + l))
  println("a constant if with an inline parameter in its second branch"); println(ifElseParam("" + l))
  println("a constant if with inline parameters in both"); println(ifBothParams("" + l, "" + new Loud("z")))
  println("an if on an inline parameter"); println(ifCondParam(yes, "" + l))
  println("an if on an inline parameter given a constant"); println(ifCondParam(true, "" + l))
  println("an ascribed hole as the condition"); println(ascribedCondition(true, "" + l, tail()))
  println("a negated hole as the condition"); println(negatedCondition(true, "" + l, tail()))
  println("an ascribed inline parameter as the condition"); println(ascribedInline(true, "" + l))
  println("a discarded ascription of a transparent call"); println(M.discarded(l, tail()))
  println("a transparent call used ascribed and plain"); println(M.both(l, tail()))
  println("a transparent condition in a quote"); println(M.choose("" + l, tail()))
  println("a plain inline condition in a quote"); println(M.choosePlain("" + l, tail()))
  println("a negated transparent condition in a quote"); println(M.chooseNot("" + l, tail()))
  println("a transparent condition conjoined with a hole"); println(M.chooseAnd(yes, "" + l, tail()))
  println("a transparent condition conjoined with a constant"); println(M.chooseAnd(true, "" + l, tail()))
  println("a transparent condition built by reflection"); println(M.reflectedYes("" + l, tail()))
