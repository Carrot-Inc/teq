// expect: 14:3: error: expected 'case', found '.'
// expect: 19:3: error: expected 'case', found '.'
// expect: 25:1: error: expected end of statement, found '.'
// expect: 32:3: error: expected end of statement, found '.'
// A line at the width of a match's indented cases that does not begin with `case` ends the cases: a selection
// there continues no case's body, a block in braces on the case's line no more than an indented one, and a
// match is continued by no selection; so with the cases at the `match`'s own width. scalac: "end of toplevel
// definition expected but '.' found" at each of the first three lines, "end of statement expected but '.' found"
// at the fourth.
def indentedBody(x: Int): String = x match
  case 1 =>
    val s = "ab"
    s
  .reverse
  case _ => "cd"

def bracedBody(x: Int): String = x match
  case 1 => { val s = "ab"; s }
  .reverse
  case _ => "cd"

val whole = 1 match
  case 1 => "ab"
  case _ => "cd"
.reverse

def atKeywordWidth(x: Int): Any =
  x match
  case 1 => "a"
  case _ =>
    "bcd"
  .length
