// A line further in than cases written at the width of the cases above them is the last case's second statement,
// where their `match` stands on a case's line after that case's block in braces. scalac prints one|zz|mx/m.
var seen = ""

def t(n: Int, w: String): Any =
  n match
  case 1 => "one"
  case _ => { w } match
  case o if o == "x" => "mx"
  case o => seen += "m"
    "zz"

@main def main(): Unit = println(s"${t(1, "a")}|${t(2, "a")}|${t(2, "x")}/$seen")
