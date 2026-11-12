// expect: 15:22: error: missing argument list for value of type String => Tone
// expect: 16:23: error: missing argument list for value of type Int => Tone
// expect: 17:21: error: missing argument list for value of type Int => Int
// expect: 18:23: error: missing argument list for value of type String => String => String
// expect: 19:23: warning: Discarded non-Unit value of type Int => Int
// A method named alone as the body of a method whose result is `Unit` is eta-expanded and its value
// dropped, a closure the typer made in statement position: an error (`Typer.adapt` under a `Unit`
// prototype calls `checkStatementPurity`, E178), an enum companion's lookup as any method; a written
// function literal there is warned of as a discarded value.
enum Tone:
  case Low, High
object Ops:
  def twice(x: Int): Int = x * 2
  def join(a: String)(b: String): String = a + b
def lookup(): Unit = Tone.valueOf
def ordinal(): Unit = Tone.fromOrdinal
def plain(): Unit = Ops.twice
def curried(): Unit = Ops.join
def written(): Unit = (x: Int) => Ops.twice(x)
