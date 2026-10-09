// `java.nio`'s byte buffers for JavaScript, written from the JDK's documented behaviour; on the
// JVM the classes are the JDK's own (`@jvmClass`), whose std bodies a macro runs in the interpreter
// (izumi-reflect's boopickle encodes a tag through them).
package java.nio:

  @javaDefined
  @jvmClass("java/nio/Buffer")
  abstract class Buffer:
    @jvm("invokevirtual java/nio/Buffer.position()I")
    def position(): Int
    @jvm("invokevirtual java/nio/Buffer.position(I)Ljava/nio/Buffer;")
    def position(p: Int): Buffer
    @jvm("invokevirtual java/nio/Buffer.limit()I")
    def limit(): Int
    @jvm("invokevirtual java/nio/Buffer.limit(I)Ljava/nio/Buffer;")
    def limit(l: Int): Buffer
    @jvm("invokevirtual java/nio/Buffer.capacity()I")
    def capacity(): Int
    @jvm("invokevirtual java/nio/Buffer.remaining()I")
    def remaining(): Int = limit() - position()
    @jvm("invokevirtual java/nio/Buffer.hasRemaining()Z")
    def hasRemaining(): Boolean = position() < limit()
    @jvm("invokevirtual java/nio/Buffer.flip()Ljava/nio/Buffer;")
    def flip(): Buffer
    @jvm("invokevirtual java/nio/Buffer.clear()Ljava/nio/Buffer;")
    def clear(): Buffer
    @jvm("invokevirtual java/nio/Buffer.rewind()Ljava/nio/Buffer;")
    def rewind(): Buffer
    @jvm("invokevirtual java/nio/Buffer.isDirect()Z")
    def isDirect(): Boolean = false
    @jvm("invokevirtual java/nio/Buffer.hasArray()Z")
    def hasArray(): Boolean

  @javaDefined
  @jvmClass("java/nio/ByteOrder")
  final class ByteOrder private[nio] (name: String):
    override def toString: String = name

  @javaDefined
  @jvmClass("java/nio/ByteOrder")
  object ByteOrder:
    private val big = new ByteOrder("BIG_ENDIAN")
    private val little = new ByteOrder("LITTLE_ENDIAN")
    @jvm("getstatic java/nio/ByteOrder.BIG_ENDIAN:Ljava/nio/ByteOrder;")
    def BIG_ENDIAN: ByteOrder = big
    @jvm("getstatic java/nio/ByteOrder.LITTLE_ENDIAN:Ljava/nio/ByteOrder;")
    def LITTLE_ENDIAN: ByteOrder = little
    @jvm("invokestatic java/nio/ByteOrder.nativeOrder()Ljava/nio/ByteOrder;")
    def nativeOrder(): ByteOrder = little

  // A byte buffer over an array, as a library reads a body through: the JDK's on the JVM.
  @javaDefined
  @jvmClass("java/nio/ByteBuffer")
  final class ByteBuffer private (buf: Array[Byte], offset: Int, private var pos: Int, private var lim: Int) extends Buffer:
    private var bigEndian = true
    @jvm("invokevirtual java/nio/ByteBuffer.array()[B")
    def array(): Array[Byte] = buf
    @jvm("invokevirtual java/nio/ByteBuffer.hasArray()Z")
    def hasArray(): Boolean = true
    @jvm("invokevirtual java/nio/ByteBuffer.arrayOffset()I")
    def arrayOffset(): Int = offset
    @jvm("invokevirtual java/nio/ByteBuffer.position()I")
    def position(): Int = pos
    @jvm("invokevirtual java/nio/ByteBuffer.position(I)Ljava/nio/ByteBuffer;")
    def position(p: Int): ByteBuffer =
      pos = p
      this
    @jvm("invokevirtual java/nio/ByteBuffer.limit()I")
    def limit(): Int = lim
    @jvm("invokevirtual java/nio/ByteBuffer.limit(I)Ljava/nio/ByteBuffer;")
    def limit(l: Int): ByteBuffer =
      lim = l
      this
    @jvm("invokevirtual java/nio/ByteBuffer.capacity()I")
    def capacity(): Int = buf.length - offset
    @jvm("invokevirtual java/nio/ByteBuffer.order()Ljava/nio/ByteOrder;")
    def order(): ByteOrder = if bigEndian then ByteOrder.BIG_ENDIAN else ByteOrder.LITTLE_ENDIAN
    @jvm("invokevirtual java/nio/ByteBuffer.order(Ljava/nio/ByteOrder;)Ljava/nio/ByteBuffer;")
    def order(bo: ByteOrder): ByteBuffer =
      bigEndian = bo eq ByteOrder.BIG_ENDIAN
      this
    @jvm("invokevirtual java/nio/ByteBuffer.get()B")
    def get(): Byte =
      val b = buf(offset + pos)
      pos += 1
      b
    @jvm("invokevirtual java/nio/ByteBuffer.get(I)B")
    def get(i: Int): Byte = buf(offset + i)
    @jvm("invokevirtual java/nio/ByteBuffer.get([B)Ljava/nio/ByteBuffer;")
    def get(dst: Array[Byte]): ByteBuffer = get(dst, 0, dst.length)
    @jvm("invokevirtual java/nio/ByteBuffer.get([BII)Ljava/nio/ByteBuffer;")
    def get(dst: Array[Byte], off: Int, len: Int): ByteBuffer =
      var i = 0
      while i < len do
        dst(off + i) = buf(offset + pos + i)
        i += 1
      pos += len
      this
    private def readInt(at: Int): Int =
      val b0 = buf(offset + at) & 0xff
      val b1 = buf(offset + at + 1) & 0xff
      val b2 = buf(offset + at + 2) & 0xff
      val b3 = buf(offset + at + 3) & 0xff
      if bigEndian then (b0 << 24) | (b1 << 16) | (b2 << 8) | b3 else (b3 << 24) | (b2 << 16) | (b1 << 8) | b0
    private def writeInt(at: Int, v: Int): Unit =
      var i = 0
      while i < 4 do
        val shift = if bigEndian then 24 - 8 * i else 8 * i
        buf(offset + at + i) = (v >>> shift).toByte
        i += 1
    @jvm("invokevirtual java/nio/ByteBuffer.getInt()I")
    def getInt(): Int =
      val v = readInt(pos)
      pos += 4
      v
    @jvm("invokevirtual java/nio/ByteBuffer.getInt(I)I")
    def getInt(i: Int): Int = readInt(i)
    @jvm("invokevirtual java/nio/ByteBuffer.getLong()J")
    def getLong(): Long =
      val a = readInt(pos).toLong & 0xffffffffL
      val b = readInt(pos + 4).toLong & 0xffffffffL
      pos += 8
      if bigEndian then (a << 32) | b else (b << 32) | a
    @jvm("invokevirtual java/nio/ByteBuffer.getLong(I)J")
    def getLong(i: Int): Long =
      val a = readInt(i).toLong & 0xffffffffL
      val b = readInt(i + 4).toLong & 0xffffffffL
      if bigEndian then (a << 32) | b else (b << 32) | a
    @jvm("invokevirtual java/nio/ByteBuffer.asLongBuffer()Ljava/nio/LongBuffer;")
    // A view over the same bytes in the order of the moment: a duplicate, whose order stays.
    def asLongBuffer(): LongBuffer = new LongBuffer(duplicate(), pos, (lim - pos) / 8)
    @jvm("invokevirtual java/nio/ByteBuffer.putInt(I)Ljava/nio/ByteBuffer;")
    def putInt(v: Int): ByteBuffer =
      writeInt(pos, v)
      pos += 4
      this
    @jvm("invokevirtual java/nio/ByteBuffer.putLong(J)Ljava/nio/ByteBuffer;")
    def putLong(v: Long): ByteBuffer =
      val hi = (v >>> 32).toInt
      val lo = v.toInt
      writeInt(pos, if bigEndian then hi else lo)
      writeInt(pos + 4, if bigEndian then lo else hi)
      pos += 8
      this
    @jvm("invokevirtual java/nio/ByteBuffer.putLong(IJ)Ljava/nio/ByteBuffer;")
    def putLong(i: Int, v: Long): ByteBuffer =
      val hi = (v >>> 32).toInt
      val lo = v.toInt
      writeInt(i, if bigEndian then hi else lo)
      writeInt(i + 4, if bigEndian then lo else hi)
      this
    @jvm("invokevirtual java/nio/ByteBuffer.put(B)Ljava/nio/ByteBuffer;")
    def put(b: Byte): ByteBuffer =
      buf(offset + pos) = b
      pos += 1
      this
    @jvm("invokevirtual java/nio/ByteBuffer.put([B)Ljava/nio/ByteBuffer;")
    def put(src: Array[Byte]): ByteBuffer =
      var i = 0
      while i < src.length do
        buf(offset + pos + i) = src(i)
        i += 1
      pos += src.length
      this
    @jvm("invokevirtual java/nio/ByteBuffer.put(Ljava/nio/ByteBuffer;)Ljava/nio/ByteBuffer;")
    def put(src: ByteBuffer): ByteBuffer =
      while src.hasRemaining() do put(src.get())
      this
    @jvm("invokevirtual java/nio/ByteBuffer.flip()Ljava/nio/ByteBuffer;")
    def flip(): ByteBuffer =
      lim = pos
      pos = 0
      this
    @jvm("invokevirtual java/nio/ByteBuffer.clear()Ljava/nio/ByteBuffer;")
    def clear(): ByteBuffer =
      pos = 0
      lim = capacity()
      this
    @jvm("invokevirtual java/nio/ByteBuffer.rewind()Ljava/nio/ByteBuffer;")
    def rewind(): ByteBuffer =
      pos = 0
      this
    @jvm("invokevirtual java/nio/ByteBuffer.duplicate()Ljava/nio/ByteBuffer;")
    def duplicate(): ByteBuffer = new ByteBuffer(buf, offset, pos, lim).order(order())
    @jvm("invokevirtual java/nio/ByteBuffer.slice()Ljava/nio/ByteBuffer;")
    def slice(): ByteBuffer = new ByteBuffer(buf, offset + pos, 0, lim - pos)

  @javaDefined
  @jvmClass("java/nio/ByteBuffer")
  object ByteBuffer:
    @jvm("invokestatic java/nio/ByteBuffer.wrap([B)Ljava/nio/ByteBuffer;")
    def wrap(array: Array[Byte]): ByteBuffer = new ByteBuffer(array, 0, 0, array.length)
    @jvm("invokestatic java/nio/ByteBuffer.wrap([BII)Ljava/nio/ByteBuffer;")
    def wrap(array: Array[Byte], offset: Int, length: Int): ByteBuffer = new ByteBuffer(array, 0, offset, offset + length)
    @jvm("invokestatic java/nio/ByteBuffer.allocate(I)Ljava/nio/ByteBuffer;")
    def allocate(capacity: Int): ByteBuffer = wrap(new Array[Byte](capacity))

  // A view of a byte buffer's remaining bytes as longs, in the buffer's order.
  @javaDefined
  @jvmClass("java/nio/LongBuffer")
  final class LongBuffer private[nio] (bytes: ByteBuffer, base: Int, count: Int) extends Buffer:
    private var pos = 0
    private var lim = count
    @jvm("invokevirtual java/nio/LongBuffer.get()J")
    def get(): Long =
      val v = get(pos)
      pos += 1
      v
    @jvm("invokevirtual java/nio/LongBuffer.get(I)J")
    def get(i: Int): Long =
      if i < 0 || i >= lim then throw new IndexOutOfBoundsException(i.toString)
      bytes.getLong(base + 8 * i)
    @jvm("invokevirtual java/nio/LongBuffer.position()I")
    def position(): Int = pos
    @jvm("invokevirtual java/nio/LongBuffer.position(I)Ljava/nio/LongBuffer;")
    def position(p: Int): LongBuffer =
      pos = p
      this
    @jvm("invokevirtual java/nio/LongBuffer.limit()I")
    def limit(): Int = lim
    @jvm("invokevirtual java/nio/LongBuffer.limit(I)Ljava/nio/LongBuffer;")
    def limit(l: Int): LongBuffer =
      lim = l
      this
    @jvm("invokevirtual java/nio/LongBuffer.capacity()I")
    def capacity(): Int = count
    @jvm("invokevirtual java/nio/LongBuffer.flip()Ljava/nio/LongBuffer;")
    def flip(): LongBuffer =
      lim = pos
      pos = 0
      this
    @jvm("invokevirtual java/nio/LongBuffer.clear()Ljava/nio/LongBuffer;")
    def clear(): LongBuffer =
      pos = 0
      lim = count
      this
    @jvm("invokevirtual java/nio/LongBuffer.rewind()Ljava/nio/LongBuffer;")
    def rewind(): LongBuffer =
      pos = 0
      this
    @jvm("invokevirtual java/nio/LongBuffer.hasArray()Z")
    def hasArray(): Boolean = false
