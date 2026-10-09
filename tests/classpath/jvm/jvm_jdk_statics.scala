// jars: scala-library
// std: lean scala-library
// A std object standing for a JDK class's statics (`@jvmClass`): `Pattern.compile` and
// `Pattern.matches` are the JDK's static methods, reached without a module instance; Java enum
// values and their final `ordinal`.
import java.util.regex.Pattern
import java.util.concurrent.TimeUnit
@main def run(): Unit =
  val p = Pattern.compile("a+b")
  println(p.matcher("aaab").matches())
  println(Pattern.matches("x*", "xx"))
  println(TimeUnit.SECONDS.ordinal)
  println(TimeUnit.valueOf("MINUTES"))
  val u: TimeUnit = TimeUnit.HOURS
  u match
    case TimeUnit.HOURS => println("hours")
    case _ => println("other")
  println(java.time.DayOfWeek.MONDAY.ordinal)
