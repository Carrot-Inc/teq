import java.nio.{ByteBuffer, CharBuffer}
import java.nio.charset.{Charset, StandardCharsets}
import scala.quoted.*

// Percent-encoding through `Charset.encode` and a `CharBuffer` at compile time, as http4s'
// `uri".."` literal does in `UriCoding.encode`, and decoding through `Charset.decode`: the
// std's bodies of the JDK's classes run in the interpreter, and the output calls the JDK's.
object Coding:
  private val hex = "0123456789ABCDEF"
  def encode(s: String, charset: Charset): String =
    val in = charset.encode(s)
    val out = CharBuffer.allocate(in.remaining() * 3)
    while in.hasRemaining do
      val b = in.get() & 0xff
      val c = b.toChar
      if c.isLetterOrDigit && b < 128 then out.put(c)
      else
        out.put('%')
        out.put(hex.charAt(b >> 4))
        out.put(hex.charAt(b & 0xf))
    out.flip()
    out.toString
  def decode(s: String, charset: Charset): String =
    val in = CharBuffer.wrap(s)
    val out = ByteBuffer.allocate(in.remaining())
    while in.hasRemaining do
      val c = in.get()
      if c == '%' then
        val x = Character.digit(in.get(), 16)
        val y = Character.digit(in.get(), 16)
        out.put(((x << 4) + y).toByte)
      else out.put(c.toByte)
    out.flip()
    charset.decode(out).toString

object Macros:
  inline def encoded(inline s: String): String = ${ encodedImpl('s) }
  def encodedImpl(s: Expr[String])(using Quotes): Expr[String] =
    val e = Coding.encode(s.valueOrAbort, StandardCharsets.UTF_8)
    Expr(e + " " + Coding.decode(e, StandardCharsets.UTF_8) + " " + "abcdef".regionMatches(2, "xcde", 1, 3))
