//> using platform jvm
// Bytes to text and back as the JDK's UTF-8 coder takes them: each malformed sequence replaced by
// one U+FFFD, by the JDK's rules of what is one (a lone continuation byte, a truncated sequence, an
// overlong form, an encoded surrogate's three bytes, a lead byte past U+10FFFF), the rest decoded;
// a lone surrogate encoded as `?`;
// ISO-8859-1 and US-ASCII byte for byte. Each result shown as its UTF-16 units.
import java.nio.charset.StandardCharsets.{UTF_8, ISO_8859_1, US_ASCII}

object Main:
  def units(s: String): String = s.map(c => Integer.toHexString(c.toInt)).mkString(" ")
  def bytes(xs: Int*): Array[Byte] = xs.map(_.toByte).toArray
  def main(args: Array[String]): Unit =
    val inputs = List(
      bytes(0x41, 0xc3, 0xa9, 0x42),
      bytes(0x80, 0x41),
      bytes(0xc3),
      bytes(0xc3, 0x41),
      bytes(0xe2, 0x82, 0x41),
      bytes(0xe2, 0x82),
      bytes(0xf0, 0x9f, 0x98, 0x80, 0x21),
      bytes(0xf0, 0x9f, 0x98),
      bytes(0xc0, 0x80),
      bytes(0xe0, 0x80, 0x80),
      bytes(0xed, 0xa0, 0x80),
      bytes(0xf4, 0x90, 0x80, 0x80),
      bytes(0xf5, 0x41),
      bytes(0xff, 0xfe, 0x41),
      bytes(0xe1, 0x80, 0xe1, 0x80, 0x80)
    )
    for b <- inputs do println(units(new String(b, UTF_8)))
    println(units(new String(bytes(0x41, 0xc3, 0xa9, 0x42, 0x43), 1, 3, UTF_8)))
    for s <- List("Aé€😀", "\ud800", "x\udc00y", "\ud800\ud800") do
      println(s.getBytes(UTF_8).map(b => Integer.toHexString(b & 0xff)).mkString(" "))
    println(units(new String(bytes(0x41, 0xe9, 0xff), ISO_8859_1)))
    println(units(new String(bytes(0x41, 0xe9, 0x7f), US_ASCII)))
    val big = ("aéb\n" * 300000).getBytes(UTF_8)
    println(big.length + " " + new String(big, UTF_8).length)
