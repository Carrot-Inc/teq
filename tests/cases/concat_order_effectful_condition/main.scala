//> using platform js
// A condition of a constant type that is no idempotent expression, folded in an expansion: scalac keeps it in front
// of the selected branch (InlineTyper.typedIf's Block(cond :: Nil, selected)), and the chain of + goes on through
// that block on the JVM, in a quote's copy as around an inline parameter.
import Macros.*

class Loud(s: String):
  override def toString(): String =
    println("  render " + s)
    s

def tail(): String =
  println("  tail")
  "!"

inline def condParam(inline s: String): String = (if Conds.cond() then s else "") + tail()

@main def main(): Unit =
  val l = new Loud("x")
  println("a call of a constant type as the condition, in a quote"); println(condHole("" + l, tail()))
  println("the same around an inline parameter"); println(condParam("" + l))
