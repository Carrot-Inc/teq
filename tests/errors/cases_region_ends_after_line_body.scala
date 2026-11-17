// expect: 15:5: error: expected 'case', found '.'
// expect: 22:3: error: expected end of statement, found '.'
// expect: 30:3: error: expected end of statement, found 'match'
// A line at the cases' width that does not begin with `case` ends the cases where the last case's body stands on
// the case's line and ends in an indented branch: a selection there continues neither the branch nor the case,
// under cases indented below their `match` and under cases at the `match`'s width, and a `match` there after the
// cases of a `catch` matches no value. scalac: "end of statement expected but '.' found" at each selection; for
// the third "Missing parameter type" at the first case of the `catch`, its cases read as a function that the
// `match` takes, and a type mismatch at each case after the `match`.
def indentedCases(x: Int): Any =
  x match
    case 1 => "a"
    case _ => if x > 5 then "b" else
      "c"
    .toUpperCase

def casesAtKeywordWidth(x: Int): Any =
  x match
  case 1 => "a"
  case _ => if x > 5 then "b" else
    "c"
  .toUpperCase

def catchCases(x: Int): Any =
  try
    if x == 1 then throw new RuntimeException("r") else "t"
  catch
  case e: RuntimeException => if x > 5 then "b" else
    "c"
  match
  case "c" => 1
  case _ => 2
