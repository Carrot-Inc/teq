// A line further in than cases written at their `match`'s width is the last case's second statement, where the
// `match` continues the indented `else` branch of an `inline if`: the other branch is not matched. scalac prints
// t|zz|m.
var seen = ""

inline def t(inline c: Boolean): Any =
  inline if c then "t"
  else
      "a"
  match
  case o if seen == "x" => "mx"
  case o => seen += "m"
    "zz"

@main def main(): Unit = println(s"${t(true)}|${t(false)}|$seen")
