import java.util.regex.Pattern

// A group index outside the pattern's groups is an error for `group`, `start` and `end`.
@main def main(): Unit =
  val m = Pattern.compile("a(b)?").matcher("xa")
  println(m.find())
  println(s"${m.start()} ${m.end()} ${m.start(1)} ${m.end(1)} ${m.group(1)}")
  for i <- List(-1, 2) do
    try println(m.start(i)) catch case _: IndexOutOfBoundsException => println(s"start $i")
    try println(m.end(i)) catch case _: IndexOutOfBoundsException => println(s"end $i")
    try println(m.group(i)) catch case _: IndexOutOfBoundsException => println(s"group $i")
