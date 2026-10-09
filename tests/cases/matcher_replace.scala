// `Matcher.replaceAll` and `replaceFirst` with group references and an escaped `$`, over the
// whole input whatever the region, as the JDK resets the matcher first.
import java.util.regex.Pattern

@main def main(): Unit =
  val p = Pattern.compile("([a-z\\d])([A-Z])")
  println(p.matcher("petNameXy2Z").replaceAll("$1_$2"))
  println(p.matcher("petNameXy2Z").replaceFirst("$1_$2"))
  val m = Pattern.compile("a+").matcher("caaab aa")
  m.region(4, 8)
  println(m.replaceAll("\\$"))
  println(Pattern.compile("x").matcher("none").replaceAll("y"))
