// expect: 11:3: error: expected end of statement, found '.'
// Cases at the width of their `match`, the last case's indented body continued by an operator that leads a line of
// the cases' width: a selection on the next line of that width continues nothing: the cases have ended there.
// scalac: "end of statement expected but '.' found" at that line.
def chained(x: Int): Any =
  x match
  case 1 => "a"
  case _ =>
    "c"
  + "z"
  .concat("!")
