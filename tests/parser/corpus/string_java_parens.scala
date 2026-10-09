// The std's `String` members stand for `java.lang.String`'s and take the Java parentheses rule:
// `s.getBytes` without the `()` scalac lets a Java method drop.
@main def run(): Unit =
  val s = "service-account"
  println(s.getBytes.length)
  println(s.trim().length)
