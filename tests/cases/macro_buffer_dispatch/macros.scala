import java.nio.{Buffer, ByteBuffer}
import scala.quoted.*

// `Buffer.limit()` is abstract under a `@jvm` template: the interpreter runs the receiver's
// implementation on every target.
object Macros:
  inline def room(inline n: Int): String = ${ roomImpl('n) }
  def roomImpl(n: Expr[Int])(using Quotes): Expr[String] =
    val buffer: Buffer = ByteBuffer.allocate(n.valueOrAbort).position(3)
    Expr(s"${buffer.limit()} ${buffer.remaining()} ${buffer.hasRemaining()}")
