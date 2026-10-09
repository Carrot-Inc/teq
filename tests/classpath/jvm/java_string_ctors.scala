// jars: scala-library
// std: lean scala-library
// `new String(buffer)` over a `StringWriter`'s `StringBuffer` or a `StringBuilder`, and
// `String(bytes, charset)` without `new`, which the std's `String` object stands for as the
// JDK's constructor.
import java.io.{PrintWriter, StringWriter}
import java.nio.charset.StandardCharsets.UTF_8

object Main:
  def main(args: Array[String]): Unit =
    val sw = new StringWriter()
    new Throwable("boom").printStackTrace(new PrintWriter(sw))
    println(new String(sw.getBuffer).linesIterator.next())
    val sb = new java.lang.StringBuilder("sb")
    println(new String(sb))
    val bytes = "héllo".getBytes(UTF_8)
    println(String(bytes, UTF_8))
