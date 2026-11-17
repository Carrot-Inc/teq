// A line further in than cases written at their `match`'s width is the last case's second statement, where the
// `match` continues the indented right side of an assignment: the match's value is the variable's. scalac prints
// a/a/m.
object Q:
  var slot: Any = "0"
  var seen = ""
  def t: Any =
    slot =
        "a"
  match
  case o if seen == "x" => "mx"
  case o => seen += "m"
    slot

@main def main(): Unit = println(s"${Q.t}/${Q.slot}/${Q.seen}")
