// expect: 10:3: error: missing parameter type
// The cases of a `catch` at the `try`'s width, the last case's body ending in an indented branch that a selection
// continues: a `match` on a line of the cases' width ends the cases and takes them as a function: no value is
// matched. scalac: "Missing parameter type" at the first case of the `catch`, and a type mismatch at each case
// after the `match`.
def chained(x: Int): Any =
  try
    if x == 1 then throw new RuntimeException("r") else "t"
  catch
  case e: RuntimeException => if x > 5 then "b" else
      "c"
    .toUpperCase
  match
  case "t" => 1
  case "C" => 3
  case _ => 2
