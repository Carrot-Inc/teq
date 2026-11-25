// `java.lang.String.format(fmt, args*)`, the JDK's static named in full, formats as `String.format` and as Java's own
// call: strings, numbers, padding, a literal percent and no arguments at all.
object Main:
  def main(args: Array[String]): Unit =
    println(java.lang.String.format("%s:%s", "a", "b"))
    println(java.lang.String.format("%d-%s", 1, "x"))
    println(java.lang.String.format("%5.2f|%x|%05d", 3.14159, 255, 42))
    println(java.lang.String.format("[%-4s][%4s]", "ab", "cd"))
    println(java.lang.String.format("100%%"))
    println(java.lang.String.format("%s", null))
    val parts = Seq("p", "q")
    println(java.lang.String.format("%s+%s", parts*))
