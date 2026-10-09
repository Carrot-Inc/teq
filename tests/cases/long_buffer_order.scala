import java.nio.{ByteBuffer, ByteOrder}

// A `LongBuffer` view keeps the byte order its buffer had when the view was made, and sees the
// buffer's later writes.
@main def run(): Unit =
  val b = ByteBuffer.allocate(16)
  b.putLong(1)
  b.flip()
  val v = b.asLongBuffer()
  b.order(ByteOrder.LITTLE_ENDIAN)
  println(v.get())
  b.putLong(0, 2)
  println(v.get(0))
  println(b.asLongBuffer().get(0))
