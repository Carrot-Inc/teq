// `new String` from bytes in a charset and from chars, as a library body decodes a buffer.
import java.nio.charset.StandardCharsets

object Main:
  def main(args: Array[String]): Unit =
    val utf8 = "héllo".getBytes(StandardCharsets.UTF_8)
    println(new String(utf8, StandardCharsets.UTF_8))
    val latin = Array[Byte](104, 105)
    println(new String(latin, StandardCharsets.ISO_8859_1))
    println(new String(Array('a', 'b', 'c'), 1, 2))
