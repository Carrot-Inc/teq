// The cases of a `match` inside the braces of an interpolated expression stand on the lines after the keyword, in
// a string and in a multi-line string. scalac prints [one] and <other>.
@main def main(): Unit =
  val n = 2
  val t = s"[${n - 1 match
    case 1 => "one"
    case _ => "other"
  }]"
  println(t)
  val s = s"""<${n match
    case 1 => "one"
    case _ => "other"
  }>"""
  println(s)
