import java.nio.ByteBuffer

// A UUID's two halves through a byte buffer and back through its view as longs.
@main def run(): Unit =
  val bb = ByteBuffer.wrap(new Array[Byte](16))
  bb.putLong(0x0123456789abcdefL)
  bb.putLong(-2L)
  val longs = ByteBuffer.wrap(bb.array()).asLongBuffer
  println(longs.get(0).toHexString)
  println(longs.get(1))
  println(longs.capacity())
