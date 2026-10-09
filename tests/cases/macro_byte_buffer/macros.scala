import java.nio.{ByteBuffer, ByteOrder}
import scala.quoted.*

// The macro encodes through `java.nio.ByteBuffer` at compile time, as izumi-reflect's boopickle
// does: the std's body of the JDK's class runs in the interpreter on every target.
object Macros:
  inline def packed(inline a: Int, inline b: Long): String = ${ packedImpl('a, 'b) }
  def packedImpl(a: Expr[Int], b: Expr[Long])(using Quotes): Expr[String] =
    val bb = ByteBuffer.allocate(12).order(ByteOrder.LITTLE_ENDIAN)
    bb.putInt(a.valueOrAbort).putLong(b.valueOrAbort)
    bb.flip()
    val bytes = new Array[Byte](bb.limit())
    bb.get(bytes)
    val big = ByteBuffer.wrap(bytes).getLong(4)
    Expr(bytes.map(x => (x & 0xff).toString).mkString(",") + " " + big)
