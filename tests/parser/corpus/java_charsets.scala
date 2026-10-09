// `java.nio.charset.Charset` and `StandardCharsets`, `String.getBytes` in each form, the byte
// constructors of `String` and `codePointCount`, against the JDK on every target.
import java.nio.charset.{Charset, StandardCharsets}

object Main:
  def main(args: Array[String]): Unit =
    val s = "héllo wörld €"
    val utf8 = s.getBytes(StandardCharsets.UTF_8)
    println(utf8.toList)
    println(s.getBytes("UTF-8").length + " " + s.getBytes().length + " " + s.getBytes(Charset.forName("utf8")).length)
    println(s.getBytes(StandardCharsets.ISO_8859_1).toList)
    println(s.getBytes("US-ASCII").toList)
    println(s.getBytes(StandardCharsets.UTF_16BE).toList.take(6))
    println(new String(utf8, StandardCharsets.UTF_8) == s)
    println(new String(utf8, "UTF-8"))
    println(new String(utf8, 0, 6, "UTF-8"))
    println(new String(s.getBytes(StandardCharsets.ISO_8859_1), StandardCharsets.ISO_8859_1))
    println(new String(s.getBytes(StandardCharsets.UTF_16), StandardCharsets.UTF_16))
    println(new String(utf8))
    println(StandardCharsets.UTF_8.name() + " " + Charset.forName("latin1").name() + " " + StandardCharsets.UTF_8.toString + " " + (Charset.forName("UTF8") == StandardCharsets.UTF_8))
    println("a😀b".codePointCount(0, 4) + " " + "abc".codePointCount(1, 3) + " " + "a😀b".getBytes("UTF-8").length)
    println(Charset.isSupported("UTF-8").toString + " " + Charset.isSupported("KLINGON") + " " + Charset.defaultCharset().name())
    try Charset.forName("KLINGON")
    catch case e: java.nio.charset.UnsupportedCharsetException => println("unsupported " + e.getMessage)
