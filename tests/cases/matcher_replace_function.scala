// Matcher.replaceAll and replaceFirst with a function of the match (JDK 9), its result read as a
// replacement string, the match a MatchResult.
@main def run(): Unit =
  val p = java.util.regex.Pattern.compile("@([a-z]+)")
  println(p.matcher("hi @bob and @al").replaceAll(m => s"<${m.group(1)}>"))
  println(p.matcher("hi @bob and @al").replaceFirst(m => m.group(1).toUpperCase))
  println(p.matcher("x @ab y").replaceAll(m => "$1$1"))
  val r: java.util.regex.MatchResult = { val m = p.matcher("@zz"); m.find(); m }
  println((r.group(1), r.start(), r.end(), r.groupCount()))
