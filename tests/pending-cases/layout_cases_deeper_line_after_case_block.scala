// A line further in than cases written at their `match`'s width is the last case's second statement, where the
// `match` continues a block in braces that is the body of a `catch`'s case on the case's line. scalac prints
// mx/ and zz/m.
var seen = ""

def t(x: String): Any =
  try throw new RuntimeException("r")
  catch case e: RuntimeException => { x }
  match
  case o if o == "x" => "mx"
  case o => seen += "m"
    "zz"

@main def main(): Unit =
  println(s"${t("x")}/$seen")
  println(s"${t("y")}/$seen")
