// expect: 9:9: error: value +: is not a member of Int
// A right-associative extension selected on a qualifier takes the qualifier as its method's first
// argument (`op(q)(a)`, dotty's `extMethodApply`): `2.+:("x")` for `extension (n: Int) def +:(s:
// String)` gives `2` to the String, fails, and is no member of Int (scalac's E008, the extension
// tried), where the reading with the qualifier as the receiver would take it.
object Syntax:
  extension (n: Int) def +:(s: String): String = s + n
import Syntax.*
val r = 2.+:("x")
