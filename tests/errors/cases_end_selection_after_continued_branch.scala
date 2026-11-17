// expect: 11:3: error: expected end of statement, found '.'
// Cases at the width of their `match`, the last case's body ending in an indented branch that a selection
// continues: a second selection on a line of the cases' width continues nothing: the cases have ended there.
// scalac: "end of statement expected but '.' found" at that line.
def chained(x: Int): Any =
  x match
  case 1 => "a"
  case _ => if x > 5 then "b" else
      "c"
    .toUpperCase
  .concat("!")
