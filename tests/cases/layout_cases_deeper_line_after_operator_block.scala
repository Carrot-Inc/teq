// A definition's indented right side, an `if` whose first branch is an indented block continued by a selection,
// then an operator leading a line at the definition's width and a `match` with its cases at that width: a line
// further in than the cases is the last case's second statement. scalac prints zz|mx|zz/m<zz>m<zz>.
object Q:
  var log = ""
  def rec(s: String): String = { log += "<" + s + ">"; s }
  def t(c: Boolean, w: String): Any =
    if c then
        w
      .trim
    else "e"
  + "z" match
  case o if o == "xz" => "mx"
  case o => log += "m"
    rec("zz")

@main def main(): Unit = println(s"${Q.t(true, "a")}|${Q.t(true, "x")}|${Q.t(false, "a")}/${Q.log}")
