// java.nio's Buffer, ByteOrder and a ByteBuffer's ordered int and long access, as boopickle's
// encoder writes through them.
import java.nio.{Buffer, ByteBuffer, ByteOrder}

object Main:
  def main(args: Array[String]): Unit =
    val le = ByteBuffer.allocate(32).order(ByteOrder.LITTLE_ENDIAN)
    le.putInt(0x01020304).putLong(-2L).put(7.toByte)
    (le: Buffer).flip()
    println(s"${le.order()} ${le.remaining()} ${le.get(0)} ${le.get(3)}")
    println(s"${le.getInt()} ${le.getLong()} ${le.get()} ${le.hasRemaining()}")
    val be = ByteBuffer.allocate(8)
    be.putInt(258)
    println(s"${be.order()} ${be.get(2)} ${be.get(3)} ${be.getInt(0)} ${be.isDirect()}")
    val comb = ByteBuffer.allocate(16).order(ByteOrder.LITTLE_ENDIAN)
    le.rewind()
    comb.put(le)
    println(s"${comb.position()} ${comb.array().take(4).mkString(",")}")
