// A line further in than the one case written at its `match`'s width is that case's second statement, where the
// `match` continues the indented `else` branch of an `inline if` and that branch is the one taken. scalac prints
// zz|zz/mm.
var seen = ""

inline def t(inline c: Boolean, w: String): Any =
  inline if c then "t"
  else
      w
  match
  case o => seen += "m"
    "zz"

@main def main(): Unit = println(s"${t(false, "a")}|${t(false, "x")}/$seen")
