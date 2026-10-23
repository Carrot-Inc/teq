// `$"` in an interpolation is a double quote and `$$` a dollar, in every interpolator, raw and triple-quoted too
// (dotty's `getStringPart`).
@main def run(): Unit =
  val x = 1
  println(s"a$"b$$c$x")
  println(raw"r$"s$$t\n")
  println(f"$"$x%d$"")
  println(s"""m$"n$$o""")
  "q$\"" match
    case s"q$$$"" => println("matched")
    case _ => println("no")
