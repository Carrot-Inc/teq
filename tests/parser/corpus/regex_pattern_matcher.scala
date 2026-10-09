// A Regex's pattern as scala-library gives it, a java.util.regex.Pattern with its matcher, whose
// groups are asked by name too.
@main def main(): Unit =
  val r = "(?i)get(.)(.*)".r
  println(r.pattern.matcher("getName").matches())
  println(r.pattern.matcher("name").matches())
  println(r.pattern.pattern())
  val m = java.util.regex.Pattern.compile("(?<c>[a-z&&[^aeiou]])(?<v>[aeiou])").matcher("xbe")
  println(s"${m.find()} ${m.group("c")} ${m.group("v")}")
  try println(m.group("w")) catch case e: IllegalArgumentException => println(e.getMessage)
  println(java.util.regex.Matcher.quoteReplacement("$1\\x"))
  def thrown(f: => Any): String = try f.toString catch case e: RuntimeException => e.getClass.getName
  println(thrown(m.group(null: String)))
  println(thrown(java.util.regex.Pattern.compile("(?<a>a)").matcher("a").group(null: String)))
  println(thrown(java.util.regex.Matcher.quoteReplacement(null)))
