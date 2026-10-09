//> using platform js
// java.net.URLEncoder as Scala.js's javalib has it: a run of characters to encode stops at a lone
// surrogate, where the JDK writes `?`; a character the charset lacks is `?` in both.
import java.net.URLEncoder
import java.nio.charset.StandardCharsets

@main def run(): Unit =
  println(URLEncoder.encode("a b&c", "UTF-8"))
  println(URLEncoder.encode("plain.text-with*under_score", "UTF-8"))
  println(URLEncoder.encode("key=value/path?q=1#frag", StandardCharsets.UTF_8))
  println(URLEncoder.encode("héllo wörld € 😀", "UTF-8"))
  println(URLEncoder.encode("héllo", "ISO-8859-1"))
  println(URLEncoder.encode("~!'()", "UTF-8"))
  println(URLEncoder.encode("a€b é&c", StandardCharsets.ISO_8859_1))
  println(URLEncoder.encode("aé&b", StandardCharsets.US_ASCII))
  println(URLEncoder.encode("a/b", StandardCharsets.UTF_16BE))
  val hi = 0xD800.toChar
  val lo = 0xDC00.toChar
  for s <- List(s"a${hi}b", s"a${hi}&b", s"a&${hi}b", s"a${lo}&b", s"a${hi}", s"$lo", s"é${hi}é", s"a$hi$hi😀b") do
    println(URLEncoder.encode(s, "UTF-8") + " | " + URLEncoder.encode(s, "ISO-8859-1"))
  try URLEncoder.encode("x", "no-such-charset")
  catch case e: java.io.UnsupportedEncodingException => println(s"unsupported: ${e.getMessage}")
