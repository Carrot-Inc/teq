// Strings made from bytes at compile time: `new String(bytes, ..)` with an offset and length, a
// charset or its name, and bytes taken back out of a `ByteBuffer`, decoded by the std's charset
// bodies on every target (a macro on the JVM target routed them to the char constructor and
// read zeros, izumi-reflect's pickled tags).
import scala.quoted.*
import java.nio.ByteBuffer
import java.nio.charset.StandardCharsets

object Macros:
  inline def decoded: String = ${ decodedImpl }
  def decodedImpl(using Quotes): Expr[String] =
    val bytes = "héllo wörld".getBytes(StandardCharsets.UTF_8)
    val latin = Array[Byte](4, 0, 1, 3, 83, 118, 99, -1)
    val bb = ByteBuffer.allocate(16)
    bb.put(4.toByte).putInt(7).put("Svc".getBytes(StandardCharsets.ISO_8859_1))
    bb.flip()
    val lines = List(
      new String(bytes, StandardCharsets.UTF_8),
      new String(bytes, "UTF-8"),
      new String(bytes, 0, 6, StandardCharsets.UTF_8),
      new String(bytes, 7, 6, "UTF-8"),
      new String(bytes),
      new String(latin, StandardCharsets.ISO_8859_1).map(_.toInt).mkString(","),
      new String(latin, 4, 3, "ISO-8859-1"),
      new String(bb.array(), bb.arrayOffset(), bb.limit(), StandardCharsets.ISO_8859_1).map(_.toInt).mkString(","),
    )
    Expr(lines.mkString("\n"))
