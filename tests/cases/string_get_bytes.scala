// `String.getBytes` as the JDK has it: a JVM `byte[]` under link mode, the lean std's array
// elsewhere.
@main def run(): Unit =
  val s = "héllo"
  println(s.getBytes().length)
  println(s.getBytes("UTF-8").mkString(","))
  println(s.getBytes(java.nio.charset.StandardCharsets.UTF_8).length)
