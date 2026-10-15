// java.net.URLDecoder on JavaScript as Scala.js's javalib has it: `+` a space, each run of `%XX` escapes the
// bytes the charset decodes, one message for a malformed escape and for bytes the charset does not decode
// exactly; tyrian's `LocationDetails` decodes its query so.
//> using platform js
import java.net.{URLDecoder, URLEncoder}
import java.nio.charset.StandardCharsets
@main def run(): Unit =
  println(URLDecoder.decode("a+b%20c%C3%A9%2Bd", StandardCharsets.UTF_8.name()))
  println(URLDecoder.decode("q=hello+world&x=%E2%82%AC", StandardCharsets.UTF_8))
  println(URLDecoder.decode(URLEncoder.encode("π / ü?&", "UTF-8"), "UTF-8"))
  for bad <- List("%zz", "%4", "abc%", "%C3") do
    try println(URLDecoder.decode(bad, "UTF-8"))
    catch case e: IllegalArgumentException => println(s"$bad: ${e.getMessage}")
  try URLDecoder.decode("x", "NO-SUCH") catch case e: java.io.UnsupportedEncodingException => println(e.getMessage)
