// The Java platform layer for JavaScript, `java.util` and `java.io`: see lang.scala.
package java.io:

  @jvmClass("java/io/Serializable")
  trait Serializable

  @jvmClass("java/io/Closeable")
  trait Closeable extends java.lang.AutoCloseable

  // A library's reader (zio-json's `RetractReader`) extends the JDK's abstract class.
  @javaDefined
  @jvmClass("java/io/Reader")
  abstract class Reader extends java.lang.Readable, Closeable:
    @jvm("invokevirtual java/io/Reader.read()I")
    def read(): Int
    @jvm("invokevirtual java/io/Reader.read([CII)I")
    def read(cbuf: Array[Char], off: Int, len: Int): Int
    @jvm("invokevirtual java/io/Reader.close()V")
    def close(): Unit

  @jvmClass("java/io/IOException")
  class IOException(message: String = null, cause: Throwable = null) extends java.lang.Exception(message, cause):
    def this(cause: Throwable) = this(if cause == null then null else cause.toString, cause)

  @jvmClass("java/io/UnsupportedEncodingException")
  class UnsupportedEncodingException(message: String = null) extends IOException(message)

  @jvmClass("java/io/EOFException")
  class EOFException(message: String = null) extends IOException(message)

  // The byte streams over JavaScript arrays: what a library reads a body through. A stream
  // class of a library extends these as it extends the JDK's.
  @javaDefined
  @jvmClass("java/io/InputStream")
  abstract class InputStream extends Closeable:
    @jvm("invokevirtual java/io/InputStream.read()I")
    def read(): Int
    @jvm("invokevirtual java/io/InputStream.read([B)I")
    def read(b: Array[Byte]): Int = read(b, 0, b.length)
    @jvm("invokevirtual java/io/InputStream.read([BII)I")
    def read(b: Array[Byte], off: Int, len: Int): Int =
      if len == 0 then 0
      else
        val first = read()
        if first == -1 then -1
        else
          b(off) = first.toByte
          var n = 1
          var done = false
          while n < len && !done do
            val c = read()
            if c == -1 then done = true
            else
              b(off + n) = c.toByte
              n += 1
          n
    @jvm("invokevirtual java/io/InputStream.readAllBytes()[B")
    def readAllBytes(): Array[Byte] =
      val out = new ByteArrayOutputStream()
      var c = read()
      while c != -1 do
        out.write(c)
        c = read()
      out.toByteArray()
    @jvm("invokevirtual java/io/InputStream.skip(J)J")
    def skip(n: Long): Long =
      var left = n
      while left > 0L && read() != -1 do left -= 1L
      n - left
    @jvm("invokevirtual java/io/InputStream.available()I")
    def available(): Int = 0
    @jvm("invokevirtual java/io/InputStream.close()V")
    def close(): Unit = ()
    @jvm("invokevirtual java/io/InputStream.markSupported()Z")
    def markSupported(): Boolean = false
    @jvm("invokevirtual java/io/InputStream.mark(I)V")
    def mark(readLimit: Int): Unit = ()
    @jvm("invokevirtual java/io/InputStream.reset()V")
    def reset(): Unit = throw new IOException("mark/reset not supported")
    @jvm("invokevirtual java/io/InputStream.transferTo(Ljava/io/OutputStream;)J")
    def transferTo(out: OutputStream): Long =
      var n = 0L
      var c = read()
      while c != -1 do
        out.write(c)
        n += 1L
        c = read()
      n

  @javaDefined
  @jvmClass("java/io/ByteArrayInputStream")
  class ByteArrayInputStream(buf: Array[Byte], offset: Int, length: Int) extends InputStream:
    def this(buf: Array[Byte]) = this(buf, 0, buf.length)
    private var pos = offset
    private val end = if offset + length > buf.length then buf.length else offset + length
    private var marked = offset
    @jvm("invokevirtual java/io/ByteArrayInputStream.read()I")
    def read(): Int =
      if pos < end then
        val b = buf(pos) & 0xff
        pos += 1
        b
      else -1
    @jvm("invokevirtual java/io/ByteArrayInputStream.read([BII)I")
    override def read(b: Array[Byte], off: Int, len: Int): Int =
      if pos >= end then -1
      else
        val n = if len > end - pos then end - pos else len
        var i = 0
        while i < n do
          b(off + i) = buf(pos + i)
          i += 1
        pos += n
        n
    @jvm("invokevirtual java/io/ByteArrayInputStream.readAllBytes()[B")
    override def readAllBytes(): Array[Byte] =
      val out = new Array[Byte](end - pos)
      var i = 0
      while pos < end do
        out(i) = buf(pos)
        i += 1
        pos += 1
      out
    @jvm("invokevirtual java/io/ByteArrayInputStream.skip(J)J")
    override def skip(n: Long): Long =
      val k = if n > (end - pos).toLong then (end - pos).toLong else if n < 0L then 0L else n
      pos += k.toInt
      k
    @jvm("invokevirtual java/io/ByteArrayInputStream.available()I")
    override def available(): Int = end - pos
    @jvm("invokevirtual java/io/ByteArrayInputStream.markSupported()Z")
    override def markSupported(): Boolean = true
    @jvm("invokevirtual java/io/ByteArrayInputStream.mark(I)V")
    override def mark(readLimit: Int): Unit = marked = pos
    @jvm("invokevirtual java/io/ByteArrayInputStream.reset()V")
    override def reset(): Unit = pos = marked

  @javaDefined
  @jvmClass("java/io/FilterInputStream")
  class FilterInputStream(protected val in: InputStream) extends InputStream:
    @jvm("invokevirtual java/io/FilterInputStream.read()I")
    def read(): Int = in.read()
    @jvm("invokevirtual java/io/FilterInputStream.read([BII)I")
    override def read(b: Array[Byte], off: Int, len: Int): Int = in.read(b, off, len)
    @jvm("invokevirtual java/io/FilterInputStream.skip(J)J")
    override def skip(n: Long): Long = in.skip(n)
    @jvm("invokevirtual java/io/FilterInputStream.available()I")
    override def available(): Int = in.available()
    @jvm("invokevirtual java/io/FilterInputStream.close()V")
    override def close(): Unit = in.close()
    @jvm("invokevirtual java/io/FilterInputStream.markSupported()Z")
    override def markSupported(): Boolean = in.markSupported()
    @jvm("invokevirtual java/io/FilterInputStream.mark(I)V")
    override def mark(readLimit: Int): Unit = in.mark(readLimit)
    @jvm("invokevirtual java/io/FilterInputStream.reset()V")
    override def reset(): Unit = in.reset()

  @javaDefined
  @jvmClass("java/io/OutputStream")
  abstract class OutputStream extends Closeable, java.io.Flushable:
    @jvm("invokevirtual java/io/OutputStream.write(I)V")
    def write(b: Int): Unit
    @jvm("invokevirtual java/io/OutputStream.write([B)V")
    def write(b: Array[Byte]): Unit = write(b, 0, b.length)
    @jvm("invokevirtual java/io/OutputStream.write([BII)V")
    def write(b: Array[Byte], off: Int, len: Int): Unit =
      var i = 0
      while i < len do
        write(b(off + i).toInt)
        i += 1
    @jvm("invokevirtual java/io/OutputStream.flush()V")
    def flush(): Unit = ()
    @jvm("invokevirtual java/io/OutputStream.close()V")
    def close(): Unit = ()

  @jvmClass("java/io/Flushable")
  trait Flushable:
    def flush(): Unit

  @javaDefined
  @jvmClass("java/io/FilterOutputStream")
  class FilterOutputStream(protected val out: OutputStream) extends OutputStream:
    @jvm("invokevirtual java/io/FilterOutputStream.write(I)V")
    def write(b: Int): Unit = out.write(b)
    @jvm("invokevirtual java/io/FilterOutputStream.flush()V")
    override def flush(): Unit = out.flush()
    @jvm("invokevirtual java/io/FilterOutputStream.close()V")
    override def close(): Unit =
      flush()
      out.close()

  // The primitives as the JDK writes them: big-endian, a string in modified UTF-8 after its
  // length in two bytes (what Scala.js's test bridge frames its messages with).
  @javaDefined
  @jvmClass("java/io/DataOutputStream")
  class DataOutputStream(out: OutputStream) extends FilterOutputStream(out):
    protected var written: Int = 0
    @jvm("invokevirtual java/io/DataOutputStream.write(I)V")
    override def write(b: Int): Unit =
      out.write(b)
      written += 1
    @jvm("invokevirtual java/io/DataOutputStream.write([BII)V")
    override def write(b: Array[Byte], off: Int, len: Int): Unit =
      out.write(b, off, len)
      written += len
    @jvm("invokevirtual java/io/DataOutputStream.writeBoolean(Z)V")
    def writeBoolean(v: Boolean): Unit = write(if v then 1 else 0)
    @jvm("invokevirtual java/io/DataOutputStream.writeByte(I)V")
    def writeByte(v: Int): Unit = write(v)
    @jvm("invokevirtual java/io/DataOutputStream.writeShort(I)V")
    def writeShort(v: Int): Unit =
      write((v >>> 8) & 0xff)
      write(v & 0xff)
    @jvm("invokevirtual java/io/DataOutputStream.writeChar(I)V")
    def writeChar(v: Int): Unit = writeShort(v)
    @jvm("invokevirtual java/io/DataOutputStream.writeInt(I)V")
    def writeInt(v: Int): Unit =
      write((v >>> 24) & 0xff)
      write((v >>> 16) & 0xff)
      write((v >>> 8) & 0xff)
      write(v & 0xff)
    @jvm("invokevirtual java/io/DataOutputStream.writeLong(J)V")
    def writeLong(v: Long): Unit =
      writeInt((v >>> 32).toInt)
      writeInt(v.toInt)
    @jvm("invokevirtual java/io/DataOutputStream.writeFloat(F)V")
    def writeFloat(v: Float): Unit = writeInt(java.lang.Float.floatToIntBits(v))
    @jvm("invokevirtual java/io/DataOutputStream.writeDouble(D)V")
    def writeDouble(v: Double): Unit = writeLong(java.lang.Double.doubleToLongBits(v))
    @jvm("invokevirtual java/io/DataOutputStream.writeBytes(Ljava/lang/String;)V")
    def writeBytes(s: String): Unit =
      var i = 0
      while i < s.length do
        write(s.charAt(i).toInt & 0xff)
        i += 1
    @jvm("invokevirtual java/io/DataOutputStream.writeChars(Ljava/lang/String;)V")
    def writeChars(s: String): Unit =
      var i = 0
      while i < s.length do
        writeChar(s.charAt(i).toInt)
        i += 1
    @jvm("invokevirtual java/io/DataOutputStream.writeUTF(Ljava/lang/String;)V")
    def writeUTF(s: String): Unit =
      var length = 0
      var i = 0
      while i < s.length do
        val c = s.charAt(i).toInt
        length += (if c >= 0x01 && c <= 0x7f then 1 else if c <= 0x7ff then 2 else 3)
        i += 1
      if length > 0xffff then throw new UTFDataFormatException("encoded string too long: " + length + " bytes")
      writeShort(length)
      i = 0
      while i < s.length do
        val c = s.charAt(i).toInt
        if c >= 0x01 && c <= 0x7f then write(c)
        else if c <= 0x7ff then
          write(0xc0 | (c >> 6))
          write(0x80 | (c & 0x3f))
        else
          write(0xe0 | (c >> 12))
          write(0x80 | ((c >> 6) & 0x3f))
          write(0x80 | (c & 0x3f))
        i += 1
    @jvm("invokevirtual java/io/DataOutputStream.size()I")
    final def size(): Int = written

  @javaDefined
  @jvmClass("java/io/DataInputStream")
  class DataInputStream(in: InputStream) extends FilterInputStream(in):
    private def next(): Int =
      val b = in.read()
      if b < 0 then throw new EOFException()
      b
    @jvm("invokevirtual java/io/DataInputStream.readFully([B)V")
    final def readFully(b: Array[Byte]): Unit = readFully(b, 0, b.length)
    @jvm("invokevirtual java/io/DataInputStream.readFully([BII)V")
    final def readFully(b: Array[Byte], off: Int, len: Int): Unit =
      var n = 0
      while n < len do
        val k = in.read(b, off + n, len - n)
        if k < 0 then throw new EOFException()
        n += k
    @jvm("invokevirtual java/io/DataInputStream.skipBytes(I)I")
    final def skipBytes(n: Int): Int =
      var skipped = 0
      while skipped < n && in.read() >= 0 do skipped += 1
      skipped
    @jvm("invokevirtual java/io/DataInputStream.readBoolean()Z")
    final def readBoolean(): Boolean = next() != 0
    @jvm("invokevirtual java/io/DataInputStream.readByte()B")
    final def readByte(): Byte = next().toByte
    @jvm("invokevirtual java/io/DataInputStream.readUnsignedByte()I")
    final def readUnsignedByte(): Int = next()
    @jvm("invokevirtual java/io/DataInputStream.readShort()S")
    final def readShort(): Short = ((next() << 8) | next()).toShort
    @jvm("invokevirtual java/io/DataInputStream.readUnsignedShort()I")
    final def readUnsignedShort(): Int = (next() << 8) | next()
    @jvm("invokevirtual java/io/DataInputStream.readChar()C")
    final def readChar(): Char = ((next() << 8) | next()).toChar
    @jvm("invokevirtual java/io/DataInputStream.readInt()I")
    final def readInt(): Int = (next() << 24) | (next() << 16) | (next() << 8) | next()
    @jvm("invokevirtual java/io/DataInputStream.readLong()J")
    final def readLong(): Long =
      val high = readInt().toLong
      val low = readInt().toLong & 0xffffffffL
      (high << 32) | low
    @jvm("invokevirtual java/io/DataInputStream.readFloat()F")
    final def readFloat(): Float = java.lang.Float.intBitsToFloat(readInt())
    @jvm("invokevirtual java/io/DataInputStream.readDouble()D")
    final def readDouble(): Double = java.lang.Double.longBitsToDouble(readLong())
    @jvm("invokevirtual java/io/DataInputStream.readUTF()Ljava/lang/String;")
    final def readUTF(): String =
      val length = readUnsignedShort()
      val out = new java.lang.StringBuilder()
      var read = 0
      while read < length do
        val a = next()
        read += 1
        if (a & 0x80) == 0 then out.append(a.toChar)
        else if (a & 0xe0) == 0xc0 then
          val b = next()
          read += 1
          if (b & 0xc0) != 0x80 then throw new UTFDataFormatException("malformed input around byte " + read)
          out.append((((a & 0x1f) << 6) | (b & 0x3f)).toChar)
        else if (a & 0xf0) == 0xe0 then
          val b = next()
          val c = next()
          read += 2
          if (b & 0xc0) != 0x80 || (c & 0xc0) != 0x80 then throw new UTFDataFormatException("malformed input around byte " + read)
          out.append((((a & 0x0f) << 12) | ((b & 0x3f) << 6) | (c & 0x3f)).toChar)
        else throw new UTFDataFormatException("malformed input around byte " + read)
      out.toString

  @jvmClass("java/io/UTFDataFormatException")
  class UTFDataFormatException(message: String = null) extends IOException(message)

  // The character writers a library's error report goes through (izumi-reflect's
  // `printStackTrace(new PrintWriter(sw))`).
  @javaDefined
  @jvmClass("java/io/Writer")
  abstract class Writer extends java.lang.Appendable, Closeable, Flushable:
    @jvm("invokevirtual java/io/Writer.write([CII)V")
    def write(cbuf: Array[Char], off: Int, len: Int): Unit
    @jvm("invokevirtual java/io/Writer.write(I)V")
    def write(c: Int): Unit = write(Array(c.toChar), 0, 1)
    @jvm("invokevirtual java/io/Writer.write(Ljava/lang/String;)V")
    def write(str: String): Unit = write(str.toCharArray, 0, str.length)
    @jvm("invokevirtual java/io/Writer.append(Ljava/lang/CharSequence;)Ljava/io/Writer;")
    def append(csq: CharSequence): Writer =
      write(String.valueOf(csq))
      this
    @jvm("invokevirtual java/io/Writer.append(C)Ljava/io/Writer;")
    def append(c: Char): Writer =
      write(c.toInt)
      this
    @jvm("invokevirtual java/io/Writer.flush()V")
    def flush(): Unit
    @jvm("invokevirtual java/io/Writer.close()V")
    def close(): Unit

  @javaDefined
  @jvmClass("java/io/StringWriter")
  class StringWriter extends Writer:
    private val buf = new java.lang.StringBuffer("")
    @jvm("invokevirtual java/io/StringWriter.write([CII)V")
    def write(cbuf: Array[Char], off: Int, len: Int): Unit = buf.append(new String(cbuf, off, len))
    @jvm("invokevirtual java/io/StringWriter.write(Ljava/lang/String;)V")
    override def write(str: String): Unit = buf.append(str)
    @jvm("invokevirtual java/io/StringWriter.getBuffer()Ljava/lang/StringBuffer;")
    def getBuffer: java.lang.StringBuffer = buf
    @jvm("invokevirtual java/io/StringWriter.flush()V")
    def flush(): Unit = ()
    @jvm("invokevirtual java/io/StringWriter.close()V")
    def close(): Unit = ()
    @jvm("invokevirtual java/io/StringWriter.toString()Ljava/lang/String;")
    override def toString: String = buf.toString

  @javaDefined
  @jvmClass("java/io/PrintWriter")
  class PrintWriter(out: Writer) extends Writer:
    @jvm("invokevirtual java/io/PrintWriter.write([CII)V")
    def write(cbuf: Array[Char], off: Int, len: Int): Unit = out.write(cbuf, off, len)
    @jvm("invokevirtual java/io/PrintWriter.write(Ljava/lang/String;)V")
    override def write(str: String): Unit = out.write(str)
    @jvm("invokevirtual java/io/PrintWriter.print(Ljava/lang/Object;)V")
    def print(x: Any): Unit = out.write(String.valueOf(x))
    @jvm("invokevirtual java/io/PrintWriter.println(Ljava/lang/Object;)V")
    def println(x: Any): Unit = out.write(String.valueOf(x) + "\n")
    @jvm("invokevirtual java/io/PrintWriter.println()V")
    def println(): Unit = out.write("\n")
    @jvm("invokevirtual java/io/PrintWriter.flush()V")
    def flush(): Unit = out.flush()
    @jvm("invokevirtual java/io/PrintWriter.close()V")
    def close(): Unit = out.close()

  @javaDefined
  @jvmClass("java/io/ByteArrayOutputStream")
  class ByteArrayOutputStream(initialSize: Int) extends OutputStream:
    def this() = this(32)
    private val buf = scala.collection.mutable.ArrayBuffer.empty[Byte]
    @jvm("invokevirtual java/io/ByteArrayOutputStream.write(I)V")
    def write(b: Int): Unit = buf += b.toByte
    @jvm("invokevirtual java/io/ByteArrayOutputStream.write([BII)V")
    override def write(b: Array[Byte], off: Int, len: Int): Unit =
      var i = 0
      while i < len do
        buf += b(off + i)
        i += 1
    @jvm("invokevirtual java/io/ByteArrayOutputStream.writeBytes([B)V")
    def writeBytes(b: Array[Byte]): Unit = write(b, 0, b.length)
    @javaDefined
    @jvm("invokevirtual java/io/ByteArrayOutputStream.toByteArray()[B")
    def toByteArray(): Array[Byte] = buf.toArray
    @javaDefined
    @jvm("invokevirtual java/io/ByteArrayOutputStream.size()I")
    def size(): Int = buf.length
    @jvm("invokevirtual java/io/ByteArrayOutputStream.reset()V")
    def reset(): Unit = buf.clear()
    @jvm("invokevirtual java/io/ByteArrayOutputStream.writeTo(Ljava/io/OutputStream;)V")
    def writeTo(out: OutputStream): Unit = out.write(buf.toArray)
    @jvm("invokevirtual java/io/ByteArrayOutputStream.toString()Ljava/lang/String;")
    override def toString: String = new String(buf.toArray, java.nio.charset.StandardCharsets.UTF_8)
    @jvm("invokevirtual java/io/ByteArrayOutputStream.toString(Ljava/lang/String;)Ljava/lang/String;")
    def toString(charsetName: String): String = new String(buf.toArray, charsetName)

  @javaDefined
  @jvmClass("java/io/PrintStream")
  final class PrintStream private ():
    @js("$printTo($0, $str($1) + \"\\n\")")
    @jvm("invokevirtual java/io/PrintStream.println(Ljava/lang/Object;)V")
    def println(x: Any): Unit
    @js("$printTo($0, \"\\n\")")
    @jvm("invokevirtual java/io/PrintStream.println()V")
    def println(): Unit
    @js("$printTo($0, $str($1))")
    @jvm("invokevirtual java/io/PrintStream.print(Ljava/lang/Object;)V")
    def print(x: Any): Unit
    @js("undefined")
    @jvm("invokevirtual java/io/PrintStream.flush()V")
    def flush(): Unit

package java.util:

  @jvmClass("java/util/Comparator")
  trait Comparator[T]:
    def compare(a: T, b: T): Int

  @javaDefined
  @jvmClass("java/util/Comparator")
  object Comparator

  @jvmClass("java/util/Iterator")
  trait Iterator[E]:
    def hasNext: Boolean
    def next(): E
    def remove(): Unit = throw new UnsupportedOperationException("remove")

  @jvmClass("java/util/RandomAccess")
  trait RandomAccess

  // The stores behind the collections below, JS primitives of their own so that the layer
  // stands under `--std=scala-library` too, where the lean std's collections are absent: a
  // hash map keyed by Scala equality (`$HMap`, as `scala.RawMap`) and a JS array.
  final class Store[K, V]
  @js("new $HMap()")
  def newStore[K, V]: Store[K, V]
  extension [K, V](s: Store[K, V])
    @js("$0.has($1)")
    def storeHas(key: K): Boolean
    @js("$0.get($1)")
    def storeGet(key: K): V
    @js("$0.set($1, $2)")
    def storeSet(key: K, value: V): Unit
    @js("$0.delete($1)")
    def storeDelete(key: K): Boolean
    @js("$0.keys()")
    def storeKeys: Array[K]
    @js("$0.values()")
    def storeValues: Array[V]
    @js("$0.m.size")
    def storeSize: Int
    @js("$0.clear()")
    def storeClear(): Unit
  @js("void $0.splice($1, 0, $2)")
  def arrayInsert[A](items: Array[A], index: Int, value: A): Unit
  @js("void $0.splice($1, 1)")
  def arrayRemove[A](items: Array[A], index: Int): Unit
  @js("void ($0.length = 0)")
  def arrayClear[A](items: Array[A]): Unit
  @js("$0.push($1)")
  def arrayPush[A](items: Array[A], value: A): Unit
  @js("[]")
  def newArray[A]: Array[A]
  // The array an `ArrayList` keeps its elements in, which a Scala buffer over the list shares.
  def arrayListStorage[A](list: ArrayList[A]): Array[A] = list.storage

  // The collections of `java.util` that library bodies build and read, over a JS array or the
  // std's hash map: what the JDK's do for the members here, on the JVM the JDK's own.
  @jvmClass("java/util/Collection")
  trait Collection[E] extends java.lang.Iterable[E]:
    def size(): Int
    def isEmpty(): Boolean = size() == 0
    def contains(o: Any): Boolean =
      val it = iterator()
      var found = false
      while !found && it.hasNext do found = Objects.equals(it.next(), o)
      found
    def add(e: E): Boolean = throw new UnsupportedOperationException("add")
    def addAll(c: Collection[? <: E]): Boolean =
      val it = c.asInstanceOf[Collection[E]].iterator()
      var changed = false
      while it.hasNext do changed = add(it.next()) | changed
      changed
    def remove(o: Any): Boolean = throw new UnsupportedOperationException("remove")
    def removeIf(test: java.util.function.Predicate[? >: E]): Boolean =
      val doomed = newArray[E]
      val it = iterator()
      while it.hasNext do
        val e = it.next()
        if test.test(e) then arrayPush(doomed, e)
      var i = 0
      while i < doomed.length do
        remove(doomed(i))
        i += 1
      doomed.length > 0
    def clear(): Unit = throw new UnsupportedOperationException("clear")
    def retainAll(c: Collection[?]): Boolean = throw new UnsupportedOperationException("retainAll")
    def removeAll(c: Collection[?]): Boolean =
      var changed = false
      val it = c.iterator()
      while it.hasNext do changed = remove(it.next()) | changed
      changed
    def containsAll(c: Collection[?]): Boolean =
      val it = c.iterator()
      var all = true
      while all && it.hasNext do all = contains(it.next())
      all
    // The JDK's defaults, written against classes the JVM has of its own: the elements from the
    // collection's iterator and size, both taken when the traversal starts.
    def spliterator(): Spliterator[E] = Spliterators.spliterator(this, 0)
    def stream(): java.util.stream.Stream[E] = java.util.stream.StreamSupport.stream(spliterator(), false)
    def toArray(): Array[AnyRef] =
      val out = newArray[AnyRef]
      val it = iterator()
      while it.hasNext do arrayPush(out, it.next().asInstanceOf[AnyRef])
      out
    // The JDK's `AbstractCollection.toArray`: the elements in `a` when it is large enough, a null
    // after them when it is larger, else in a new array of its kind. `size()` is a hint: the
    // iterator is read to its end, the array grown when it gives more, trimmed or copied back into
    // `a` when it gives fewer.
    def toArray[T](a: Array[T]): Array[T] =
      val n = size()
      if a == null then throw new NullPointerException()
      var r = if a.length >= n then a else Arrays.copyOf(a, n)
      val it = iterator()
      var i = 0
      while i < r.length do
        if !it.hasNext then
          if r eq a then
            r(i) = null.asInstanceOf[T]
            return a
          if a.length < i then return Arrays.copyOf(r, i)
          System.arraycopy(r, 0, a, 0, i)
          if a.length > i then a(i) = null.asInstanceOf[T]
          return a
        r(i) = it.next().asInstanceOf[T]
        i += 1
      var length = r.length
      while it.hasNext do
        if i == length then
          length = length + (length >> 1) + 1
          r = Arrays.copyOf(r, length)
        r(i) = it.next().asInstanceOf[T]
        i += 1
      if i == length then r else Arrays.copyOf(r, i)
    override def toString: String =
      val sb = new java.lang.StringBuilder("[")
      val it = iterator()
      var first = true
      while it.hasNext do
        if !first then sb.append(", ")
        first = false
        sb.append(it.next())
      sb.append("]").toString

  @jvmClass("java/util/List")
  trait List[E] extends Collection[E]:
    def get(index: Int): E
    // In the list's order, as the JDK's lists' (`ORDERED`).
    override def spliterator(): Spliterator[E] = Spliterators.spliterator(this, Spliterator.ORDERED)
    def set(index: Int, e: E): E = throw new UnsupportedOperationException("set")
    def add(e: E): Boolean = throw new UnsupportedOperationException("add")
    def add(index: Int, e: E): Unit = throw new UnsupportedOperationException("add")
    def remove(index: Int): E = throw new UnsupportedOperationException("remove")
    def indexOf(o: Any): Int =
      var i = 0
      val n = size()
      while i < n do
        if Objects.equals(get(i), o) then return i
        i += 1
      -1
    override def contains(o: Any): Boolean = indexOf(o) >= 0
    def subList(from: Int, to: Int): List[E] =
      val out = new ArrayList[E]()
      var i = from
      while i < to do
        out.add(get(i))
        i += 1
      out
    def iterator(): Iterator[E] = new ListIterator(this)
    override def equals(that: Any): Boolean = that match
      case l: List[?] =>
        if l.size() != size() then false
        else
          var i = 0
          var same = true
          while same && i < size() do
            same = Objects.equals(get(i), l.get(i))
            i += 1
          same
      case _ => false
    override def hashCode: Int =
      var h = 1
      var i = 0
      while i < size() do
        h = 31 * h + Objects.hashCode(get(i))
        i += 1
      h

  @jvmClass("java/util/List")
  object List:
    @jvm("rt $1:L rtcall arraysAsList(Ljava/lang/Object;)Ljava/lang/Object; checkcast java/util/List")
    def of[E](elems: E*): List[E] = new ArrayList(untaggedArray(iterableToArray(elems)))

  private final class ListIterator[E](list: List[E]) extends Iterator[E]:
    private var i = 0
    def hasNext: Boolean = i < list.size()
    def next(): E =
      if i >= list.size() then throw new NoSuchElementException()
      i += 1
      list.get(i - 1)

  @jvmClass("java/util/ArrayList")
  class ArrayList[E](private val items: Array[E]) extends List[E], RandomAccess:
    private[util] def storage: Array[E] = items
    def this() = this(newArray[E])
    def this(initialCapacity: Int) = this(newArray[E])
    def this(c: Collection[? <: E]) =
      this(newArray[E])
      addAll(c.asInstanceOf[Collection[E]])
    def size(): Int = items.length
    def get(index: Int): E =
      if index < 0 || index >= items.length then throw new IndexOutOfBoundsException("Index " + index + " out of bounds for length " + items.length)
      items(index)
    override def set(index: Int, e: E): E =
      val old = get(index)
      items(index) = e
      old
    override def add(e: E): Boolean =
      arrayPush(items, e)
      true
    override def add(index: Int, e: E): Unit = arrayInsert(items, index, e)
    override def remove(index: Int): E =
      val old = get(index)
      arrayRemove(items, index)
      old
    override def remove(o: Any): Boolean =
      val i = indexOf(o)
      if i >= 0 then arrayRemove(items, i)
      i >= 0
    override def clear(): Unit = arrayClear(items)
    override def iterator(): Iterator[E] = new ListIterator(this)

  @jvmClass("java/util/Set")
  trait Set[E] extends Collection[E]:
    // Each element once, as the JDK's sets' (`DISTINCT`).
    override def spliterator(): Spliterator[E] = Spliterators.spliterator(this, Spliterator.DISTINCT)

  @jvmClass("java/util/HashSet")
  class HashSet[E](private val raw: Store[E, Boolean]) extends Set[E]:
    def this() = this(newStore[E, Boolean])
    def this(initialCapacity: Int) = this(newStore[E, Boolean])
    def this(c: Collection[? <: E]) =
      this(newStore[E, Boolean])
      addAll(c.asInstanceOf[Collection[E]])
    def size(): Int = raw.storeSize
    override def contains(o: Any): Boolean = raw.storeHas(o.asInstanceOf[E])
    override def add(e: E): Boolean =
      if raw.storeHas(e) then false
      else
        raw.storeSet(e, true)
        true
    override def remove(o: Any): Boolean = raw.storeDelete(o.asInstanceOf[E])
    override def clear(): Unit = raw.storeClear()
    def iterator(): Iterator[E] = new ArrayIterator(raw.storeKeys)
    override def retainAll(c: Collection[?]): Boolean =
      var changed = false
      val keys = raw.storeKeys
      var i = 0
      while i < keys.length do
        if !c.contains(keys(i)) then
          raw.storeDelete(keys(i))
          changed = true
        i += 1
      changed

  private final class ArrayIterator[E](items: Array[E]) extends Iterator[E]:
    private var i = 0
    def hasNext: Boolean = i < items.length
    def next(): E =
      if i >= items.length then throw new NoSuchElementException()
      i += 1
      items(i - 1)

  @jvmClass("java/util/Map")
  trait Map[K, V]:
    def size(): Int
    def isEmpty(): Boolean = size() == 0
    def get(key: Any): V
    def containsKey(key: Any): Boolean
    def put(key: K, value: V): V
    def remove(key: Any): V
    def putAll(m: Map[? <: K, ? <: V]): Unit =
      val it = m.asInstanceOf[Map[K, V]].entrySet().iterator()
      while it.hasNext do
        val e = it.next()
        put(e.getKey, e.getValue)
    def keySet(): Set[K]
    def values(): Collection[V]
    def entrySet(): Set[Map.Entry[K, V]]
    def containsValue(value: Any): Boolean = values().contains(value)
    def getOrDefault(key: Any, default: V): V = if containsKey(key) then get(key) else default
    def computeIfAbsent(key: K, mapping: java.util.function.Function[K, V]): V =
      val present = get(key)
      if present != null then present
      else
        val computed = mapping.apply(key)
        if computed != null then put(key, computed)
        computed
    def clear(): Unit

  @jvmClass("java/util/Map")
  object Map:
    @jvmClass("java/util/Map$Entry")
    trait Entry[K, V]:
      def getKey: K
      def getValue: V
      def setValue(value: V): V

  @jvmClass("java/util/HashMap")
  class HashMap[K, V](private val raw: Store[K, V]) extends Map[K, V]:
    def this() = this(newStore[K, V])
    def this(initialCapacity: Int) = this(newStore[K, V])
    def this(m: Map[? <: K, ? <: V]) =
      this(newStore[K, V])
      putAll(m)
    def size(): Int = raw.storeSize
    def get(key: Any): V = if raw.storeHas(key.asInstanceOf[K]) then raw.storeGet(key.asInstanceOf[K]) else null.asInstanceOf[V]
    def containsKey(key: Any): Boolean = raw.storeHas(key.asInstanceOf[K])
    def put(key: K, value: V): V =
      val old = get(key)
      raw.storeSet(key, value)
      old
    def remove(key: Any): V =
      val old = get(key)
      raw.storeDelete(key.asInstanceOf[K])
      old
    def clear(): Unit = raw.storeClear()
    def keySet(): Set[K] =
      val out = new HashSet[K]()
      val keys = raw.storeKeys
      var i = 0
      while i < keys.length do
        out.add(keys(i))
        i += 1
      out
    def values(): Collection[V] = new ArrayList[V](raw.storeValues)
    // `AbstractMap.toString`'s `{k=v, ...}`.
    override def toString: String = SortedOps.mapString(this)
    def entrySet(): Set[Map.Entry[K, V]] =
      val out = new EntrySet[K, V]()
      val keys = raw.storeKeys
      var i = 0
      while i < keys.length do
        out.entries.add(new AbstractMap.SimpleImmutableEntry[K, V](keys(i), raw.storeGet(keys(i))))
        i += 1
      out

  private final class EntrySet[K, V] extends Set[Map.Entry[K, V]]:
    val entries: ArrayList[Map.Entry[K, V]] = new ArrayList[Map.Entry[K, V]]()
    def size(): Int = entries.size()
    def iterator(): Iterator[Map.Entry[K, V]] = entries.iterator()

  @jvmClass("java/util/AbstractMap")
  abstract class AbstractMap[K, V] extends Map[K, V]

  // Keyed by reference (`eq`), in the order of insertion: the JDK's map of that name, which a
  // library keeps the objects it has visited in (munit's `StackTraces`).
  @jvmClass("java/util/IdentityHashMap")
  class IdentityHashMap[K, V] extends AbstractMap[K, V]:
    private val keys: Array[K] = newArray[K]
    private val vals: Array[V] = newArray[V]
    private def indexOf(key: Any): Int =
      var i = 0
      while i < keys.length && !(keys(i).asInstanceOf[AnyRef] eq key.asInstanceOf[AnyRef]) do i += 1
      if i < keys.length then i else -1
    def size(): Int = keys.length
    def get(key: Any): V =
      val i = indexOf(key)
      if i < 0 then null.asInstanceOf[V] else vals(i)
    def containsKey(key: Any): Boolean = indexOf(key) >= 0
    def put(key: K, value: V): V =
      val i = indexOf(key)
      if i >= 0 then
        val old = vals(i)
        vals(i) = value
        old
      else
        arrayPush(keys, key)
        arrayPush(vals, value)
        null.asInstanceOf[V]
    def remove(key: Any): V =
      val i = indexOf(key)
      if i < 0 then null.asInstanceOf[V]
      else
        val old = vals(i)
        arrayRemove(keys, i)
        arrayRemove(vals, i)
        old
    def clear(): Unit =
      arrayClear(keys)
      arrayClear(vals)
    // The views are the map's: a removal or a clear through one is the map's, and none adds.
    def keySet(): Set[K] = new IdentityKeys(this)
    def values(): Collection[V] = new IdentityValues(this)
    def entrySet(): Set[Map.Entry[K, V]] = new IdentityEntries(this)
    private[util] def keyAt(i: Int): K = keys(i)
    private[util] def valueAt(i: Int): V = vals(i)
    private[util] def setValueAt(i: Int, value: V): V =
      val old = vals(i)
      vals(i) = value
      old
    private[util] def removeAt(i: Int): Unit =
      arrayRemove(keys, i)
      arrayRemove(vals, i)
    private[util] def hasMapping(key: Any, value: Any): Boolean =
      val i = indexOf(key)
      i >= 0 && (vals(i).asInstanceOf[AnyRef] eq value.asInstanceOf[AnyRef])
    private[util] def removeMapping(key: Any, value: Any): Boolean =
      val had = hasMapping(key, value)
      if had then removeAt(indexOf(key))
      had
    private[util] def indexOfValue(value: Any): Int =
      var i = 0
      while i < vals.length && !(vals(i).asInstanceOf[AnyRef] eq value.asInstanceOf[AnyRef]) do i += 1
      if i < vals.length then i else -1

  // An iterator over the positions of an identity map, whose `remove` removes the mapping it
  // gave last.
  private abstract class IdentityIterator[K, V, E](map: IdentityHashMap[K, V]) extends Iterator[E]:
    private var cursor = 0
    private var last = -1
    def at(i: Int): E
    def hasNext: Boolean = cursor < map.size()
    def next(): E =
      if cursor >= map.size() then throw new NoSuchElementException()
      last = cursor
      cursor += 1
      at(last)
    override def remove(): Unit =
      if last < 0 then throw new IllegalStateException()
      map.removeAt(last)
      cursor = last
      last = -1

  private final class IdentityKeys[K, V](map: IdentityHashMap[K, V]) extends Set[K]:
    def size(): Int = map.size()
    override def contains(o: Any): Boolean = map.containsKey(o)
    override def remove(o: Any): Boolean =
      val had = map.containsKey(o)
      map.remove(o)
      had
    override def clear(): Unit = map.clear()
    def iterator(): Iterator[K] = new IdentityIterator[K, V, K](map):
      def at(i: Int): K = map.keyAt(i)

  private final class IdentityValues[K, V](map: IdentityHashMap[K, V]) extends Collection[V]:
    def size(): Int = map.size()
    override def contains(o: Any): Boolean = map.indexOfValue(o) >= 0
    override def remove(o: Any): Boolean =
      val i = map.indexOfValue(o)
      if i >= 0 then map.removeAt(i)
      i >= 0
    override def clear(): Unit = map.clear()
    def iterator(): Iterator[V] = new IdentityIterator[K, V, V](map):
      def at(i: Int): V = map.valueAt(i)

  // Membership and removal by a mapping of the same key and value, by reference, as the JDK's.
  private final class IdentityEntries[K, V](map: IdentityHashMap[K, V]) extends Set[Map.Entry[K, V]]:
    def size(): Int = map.size()
    override def clear(): Unit = map.clear()
    override def contains(o: Any): Boolean = o match
      case e: Map.Entry[?, ?] => map.hasMapping(e.getKey, e.getValue)
      case _ => false
    override def remove(o: Any): Boolean = o match
      case e: Map.Entry[?, ?] => map.removeMapping(e.getKey, e.getValue)
      case _ => false
    def iterator(): Iterator[Map.Entry[K, V]] = new IdentityIterator[K, V, Map.Entry[K, V]](map):
      def at(i: Int): Map.Entry[K, V] = new IdentityEntry(map, map.keyAt(i))

  // An entry of an identity map that reads and writes the map under its key.
  private final class IdentityEntry[K, V](map: IdentityHashMap[K, V], key: K) extends Map.Entry[K, V]:
    def getKey: K = key
    def getValue: V = map.get(key)
    def setValue(value: V): V = map.put(key, value)
    override def equals(o: Any): Boolean = o match
      case e: Map.Entry[?, ?] => (e.getKey.asInstanceOf[AnyRef] eq key.asInstanceOf[AnyRef]) && (e.getValue.asInstanceOf[AnyRef] eq getValue.asInstanceOf[AnyRef])
      case _ => false
    override def hashCode: Int = System.identityHashCode(key.asInstanceOf[AnyRef]) ^ System.identityHashCode(getValue.asInstanceOf[AnyRef])

  // A set over the keys of a map, whose `add` puts `true` under the element.
  private final class SetFromMap[E](map: Map[E, java.lang.Boolean]) extends Set[E]:
    def size(): Int = map.size()
    override def contains(o: Any): Boolean = map.containsKey(o)
    override def add(e: E): Boolean = map.put(e, java.lang.Boolean.valueOf(true)) == null
    override def remove(o: Any): Boolean = map.remove(o) != null
    override def clear(): Unit = map.clear()
    def iterator(): Iterator[E] =
      val out = new ArrayList[E]()
      val it = map.entrySet().iterator()
      while it.hasNext do out.add(it.next().getKey)
      out.iterator()

  @jvmClass("java/util/AbstractMap")
  object AbstractMap:
    // The JDK's `a == null ? b == null : a.equals(b)`, by which its maps and entries compare
    // values: a box equals a box of its own kind alone (on JavaScript the numbers but `Long` are
    // one kind), a floating point as its bits would (`-0.0` is not `0.0`, a NaN is itself), an
    // array only itself, and any other object answers by its `equals`, called on `a` even when `b`
    // is `a` itself, where Scala's `==` would not call it. Where `Objects.equals` here is Scala's
    // `==`, these agree with `Objects.hashCode`.
    private[util] def valueEquals(a: Any, b: Any): Boolean = a match
      case x: Double => b match
        case y: Double => if x != x then y != y else x == y && 1 / x == 1 / y
        case _ => false
      case x: Float => b match
        case y: Float => if x != x then y != y else x == y && 1 / x == 1 / y
        case _ => false
      case _: Long => b.isInstanceOf[Long] && a == b
      case _: Int => b.isInstanceOf[Int] && a == b
      case _: Short => b.isInstanceOf[Short] && a == b
      case _: Byte => b.isInstanceOf[Byte] && a == b
      case _: Char => b.isInstanceOf[Char] && a == b
      case _: Array[?] => a.asInstanceOf[AnyRef] eq b.asInstanceOf[AnyRef]
      case null => b == null
      case _ => objectEquals(a, b)
    // `a.equals(b)` as a call of `a`'s `equals`, which `a.equals(b)` written on a value is not
    // here (it answers as Scala's `==`, true for `a` itself without the call). On JavaScript an
    // object without an `equals` of its own is equal to itself alone, as under `Object.equals`.
    @js("($1 != null && typeof $1.equals === \"function\" ? $1.equals($2) : $1 === $2)")
    private def objectEquals(a: Any, b: Any): Boolean = a.asInstanceOf[EqualsCall].equals(b)
    // What `objectEquals` calls `equals` through, a member call rather than an equality.
    private trait EqualsCall:
      def equals(o: Any): Boolean
    // A copy of a mapping, which `setValue` cannot change; equal to any entry of the same key and
    // value, as the JDK's.
    @jvmClass("java/util/AbstractMap$SimpleImmutableEntry")
    class SimpleImmutableEntry[K, V](key: K, value: V) extends Map.Entry[K, V]:
      def this(e: Map.Entry[? <: K, ? <: V]) = this(e.getKey, e.getValue)
      def getKey: K = key
      def getValue: V = value
      def setValue(value: V): V = throw new UnsupportedOperationException()
      override def equals(o: Any): Boolean = o match
        case e: Map.Entry[?, ?] => AbstractMap.valueEquals(key, e.getKey) && AbstractMap.valueEquals(value, e.getValue)
        case _ => false
      override def hashCode: Int = Objects.hashCode(key) ^ Objects.hashCode(value)
      override def toString: String = "" + key + "=" + value

  @jvmClass("java/util/Collections")
  final class Collections private ()

  @jvmClass("java/util/Collections")
  object Collections:
    def unmodifiableList[T](list: List[? <: T]): List[T] = list.asInstanceOf[List[T]]
    def unmodifiableSet[T](set: Set[? <: T]): Set[T] = set.asInstanceOf[Set[T]]
    def unmodifiableMap[K, V](map: Map[? <: K, ? <: V]): Map[K, V] = map.asInstanceOf[Map[K, V]]
    def emptyList[T](): List[T] = new ArrayList[T]()
    def emptySet[T](): Set[T] = new HashSet[T]()
    def emptyMap[K, V](): Map[K, V] = new HashMap[K, V]()
    def emptyIterator[T](): Iterator[T] = new ArrayIterator(newArray[T])
    def newSetFromMap[E](map: Map[E, java.lang.Boolean]): Set[E] = new SetFromMap(map)
    @jvm("rt $1:L $2:L rtcall collectionsAddAll(Ljava/lang/Object;Ljava/lang/Object;)Z")
    def addAll[T](c: Collection[? >: T], elements: T*): Boolean =
      var changed = false
      for e <- elements do if c.asInstanceOf[Collection[T]].add(e) then changed = true
      changed
    def singletonList[T](o: T): List[T] =
      val out = new ArrayList[T]()
      out.add(o)
      out
    def sort[T <: Comparable[? >: T]](list: List[T]): Unit =
      val items = list.toArray(newArray[T])
      Arrays.sort(items)
      var i = 0
      while i < items.length do
        list.set(i, items(i))
        i += 1
    def sort[T](list: List[T], c: Comparator[? >: T]): Unit =
      val items = list.toArray(newArray[T])
      Arrays.sort(items, c)
      var i = 0
      while i < items.length do
        list.set(i, items(i))
        i += 1

  @javaDefined
  @jvmClass("java/util/Objects")
  final class Objects private ()

  @javaDefined
  @jvmClass("java/util/Objects")
  object Objects:
    @js("$eq($1, $2)")
    @jvm("invokestatic java/util/Objects.equals(Ljava/lang/Object;Ljava/lang/Object;)Z")
    def equals(a: Any, b: Any): Boolean
    @js("($1 === null ? 0 : $hashCode($1))")
    @jvm("invokestatic java/util/Objects.hashCode(Ljava/lang/Object;)I")
    def hashCode(x: Any): Int
    @js("$requireNonNull($1)")
    @jvm("invokestatic java/util/Objects.requireNonNull(Ljava/lang/Object;)Ljava/lang/Object;")
    def requireNonNull[T](x: T): T
    @js("$requireNonNull($1)")
    @jvm("invokestatic java/util/Objects.requireNonNull(Ljava/lang/Object;Ljava/lang/String;)Ljava/lang/Object;")
    def requireNonNull[T](x: T, message: String): T
    @js("($1 === null ? \"null\" : $str($1))")
    @jvm("invokestatic java/util/Objects.toString(Ljava/lang/Object;)Ljava/lang/String;")
    def toString(x: Any): String
    @js("($1 === null)")
    @jvm("invokestatic java/util/Objects.isNull(Ljava/lang/Object;)Z")
    def isNull(x: Any): Boolean
    @js("($1 !== null)")
    @jvm("invokestatic java/util/Objects.nonNull(Ljava/lang/Object;)Z")
    def nonNull(x: Any): Boolean
    // Arrays compare element by element, nested ones deeply; anything else by `equals`, so a
    // boxed `1` is not a boxed `1L`, and floating points compare by their bits (`-0.0` is not
    // `0.0`, a NaN is itself).
    @jvm("invokestatic java/util/Objects.deepEquals(Ljava/lang/Object;Ljava/lang/Object;)Z")
    def deepEquals(a: Any, b: Any): Boolean = (a, b) match
      case (x: Array[?], y: Array[?]) =>
        x.length == y.length && {
          var i = 0
          while i < x.length && deepEquals(x(i), y(i)) do i += 1
          i == x.length
        }
      case _ if boxedKind(a) != boxedKind(b) => false
      case (x: Double, y: Double) => java.lang.Double.doubleToLongBits(x) == java.lang.Double.doubleToLongBits(y)
      case (x: Float, y: Float) => java.lang.Float.floatToIntBits(x) == java.lang.Float.floatToIntBits(y)
      case _ => a == b
    // The box a value has on the JVM, which `equals` compares first: 1 is no 1L.
    private def boxedKind(x: Any): Int = x match
      case _: Long => 1
      case _: Char => 2
      case _: Boolean => 3
      case _: Int => 4
      case _: Short => 5
      case _: Byte => 6
      case _: Float => 7
      case _: Double => 8
      case _ => 0

  @javaDefined
  @jvmClass("java/util/Arrays")
  final class Arrays private ()

  @javaDefined
  @jvmClass("java/util/Arrays")
  object Arrays:
    @js("$arrayCopyOf($1, $2)")
    @jvm("rt $1:L $2:I rtcall linkedArrayCopyOf(Ljava/lang/Object;I)Ljava/lang/Object; cast_result")
    def copyOf[T](a: Array[T], length: Int): Array[T]
    @js("$arrayCopyOfRange($1, $2, $3)")
    @jvm("rt $1:L $2:I $3:I rtcall linkedArrayCopyOfRange(Ljava/lang/Object;II)Ljava/lang/Object; cast_result")
    def copyOfRange[T](a: Array[T], from: Int, to: Int): Array[T]
    @js("$arrayCopyOf($1, $2)")
    @jvm("$1:[Ljava/lang/Object; $2:I $3 invokestatic java/util/Arrays.copyOf([Ljava/lang/Object;ILjava/lang/Class;)[Ljava/lang/Object; cast_result")
    def copyOf[T, U](a: Array[U], length: Int, newType: Class[?]): Array[T]
    // The JDK's overloads per primitive array: a copy pads with the zero of its element type,
    // which an array of this std does not carry.
    @jvm("rt $1:L $2:I rtcall linkedArrayCopyOf(Ljava/lang/Object;I)Ljava/lang/Object; cast_result")
    def copyOf(a: Array[Boolean], length: Int): Array[Boolean] = copyOfPadded(a, 0, length, false)
    @jvm("rt $1:L $2:I $3:I rtcall linkedArrayCopyOfRange(Ljava/lang/Object;II)Ljava/lang/Object; cast_result")
    def copyOfRange(a: Array[Boolean], from: Int, to: Int): Array[Boolean] = copyOfPadded(a, from, to, false)
    @jvm("rt $1:L $2:I rtcall linkedArrayCopyOf(Ljava/lang/Object;I)Ljava/lang/Object; cast_result")
    def copyOf(a: Array[Byte], length: Int): Array[Byte] = copyOfPadded(a, 0, length, 0.toByte)
    @jvm("rt $1:L $2:I $3:I rtcall linkedArrayCopyOfRange(Ljava/lang/Object;II)Ljava/lang/Object; cast_result")
    def copyOfRange(a: Array[Byte], from: Int, to: Int): Array[Byte] = copyOfPadded(a, from, to, 0.toByte)
    @jvm("rt $1:L $2:I rtcall linkedArrayCopyOf(Ljava/lang/Object;I)Ljava/lang/Object; cast_result")
    def copyOf(a: Array[Short], length: Int): Array[Short] = copyOfPadded(a, 0, length, 0.toShort)
    @jvm("rt $1:L $2:I $3:I rtcall linkedArrayCopyOfRange(Ljava/lang/Object;II)Ljava/lang/Object; cast_result")
    def copyOfRange(a: Array[Short], from: Int, to: Int): Array[Short] = copyOfPadded(a, from, to, 0.toShort)
    @jvm("rt $1:L $2:I rtcall linkedArrayCopyOf(Ljava/lang/Object;I)Ljava/lang/Object; cast_result")
    def copyOf(a: Array[Char], length: Int): Array[Char] = copyOfPadded(a, 0, length, 0.toChar)
    @jvm("rt $1:L $2:I $3:I rtcall linkedArrayCopyOfRange(Ljava/lang/Object;II)Ljava/lang/Object; cast_result")
    def copyOfRange(a: Array[Char], from: Int, to: Int): Array[Char] = copyOfPadded(a, from, to, 0.toChar)
    @jvm("rt $1:L $2:I rtcall linkedArrayCopyOf(Ljava/lang/Object;I)Ljava/lang/Object; cast_result")
    def copyOf(a: Array[Int], length: Int): Array[Int] = copyOfPadded(a, 0, length, 0)
    @jvm("rt $1:L $2:I $3:I rtcall linkedArrayCopyOfRange(Ljava/lang/Object;II)Ljava/lang/Object; cast_result")
    def copyOfRange(a: Array[Int], from: Int, to: Int): Array[Int] = copyOfPadded(a, from, to, 0)
    @jvm("rt $1:L $2:I rtcall linkedArrayCopyOf(Ljava/lang/Object;I)Ljava/lang/Object; cast_result")
    def copyOf(a: Array[Long], length: Int): Array[Long] = copyOfPadded(a, 0, length, 0L)
    @jvm("rt $1:L $2:I $3:I rtcall linkedArrayCopyOfRange(Ljava/lang/Object;II)Ljava/lang/Object; cast_result")
    def copyOfRange(a: Array[Long], from: Int, to: Int): Array[Long] = copyOfPadded(a, from, to, 0L)
    @jvm("rt $1:L $2:I rtcall linkedArrayCopyOf(Ljava/lang/Object;I)Ljava/lang/Object; cast_result")
    def copyOf(a: Array[Float], length: Int): Array[Float] = copyOfPadded(a, 0, length, 0.0f)
    @jvm("rt $1:L $2:I $3:I rtcall linkedArrayCopyOfRange(Ljava/lang/Object;II)Ljava/lang/Object; cast_result")
    def copyOfRange(a: Array[Float], from: Int, to: Int): Array[Float] = copyOfPadded(a, from, to, 0.0f)
    @jvm("rt $1:L $2:I rtcall linkedArrayCopyOf(Ljava/lang/Object;I)Ljava/lang/Object; cast_result")
    def copyOf(a: Array[Double], length: Int): Array[Double] = copyOfPadded(a, 0, length, 0.0)
    @jvm("rt $1:L $2:I $3:I rtcall linkedArrayCopyOfRange(Ljava/lang/Object;II)Ljava/lang/Object; cast_result")
    def copyOfRange(a: Array[Double], from: Int, to: Int): Array[Double] = copyOfPadded(a, from, to, 0.0)
    private def copyOfPadded[T](a: Array[T], from: Int, to: Int, zero: T): Array[T] =
      if from > to then throw new IllegalArgumentException(s"$from > $to")
      if from < 0 || from > a.length then throw new ArrayIndexOutOfBoundsException(s"Array index out of range: $from")
      val out = scala.newArray[T](to - from, zero)
      var i = from
      while i < to && i < a.length do
        out(i - from) = a(i)
        i += 1
      out
    @js("$1.fill($2)")
    @jvm("rt $1:L $2:L rtcall arraysFillAll(Ljava/lang/Object;Ljava/lang/Object;)V")
    def fill[T](a: Array[T], value: T): Unit
    @js("$1.fill($2)")
    @jvm("rt $1:L $2:L rtcall arraysFillAll(Ljava/lang/Object;Ljava/lang/Object;)V")
    def fill(a: Array[AnyRef], value: Unit): Unit
    @js("$1.fill($4, $2, $3)")
    @jvm("rt $1:L $2:I $3:I $4:L rtcall arraysFill(Ljava/lang/Object;IILjava/lang/Object;)V")
    def fill[T](a: Array[T], from: Int, to: Int, value: T): Unit
    @js("$sortArray($1, $2)")
    @jvm("rt $1:L $2:L rtcall arraysSort(Ljava/lang/Object;Ljava/lang/Object;)V")
    def sort[T](a: Array[T], comparator: Comparator[? >: T]): Unit
    @js("$sortArray($1, null)")
    @jvm("rt $1:L rtcall arraysSortNatural(Ljava/lang/Object;)V")
    def sort[T](a: Array[T]): Unit
    @js("$sortRange($1, $2, $3, null)")
    @jvm("rt $1:L $2:I $3:I rtcall arraysSortRange(Ljava/lang/Object;II)V")
    def sort[T](a: Array[T], from: Int, to: Int): Unit
    @js("$sortRange($1, $2, $3, $4)")
    @jvm("rt $1:L $2:I $3:I $4:L rtcall arraysSortRangeWith(Ljava/lang/Object;IILjava/lang/Object;)V")
    def sort[T](a: Array[T], from: Int, to: Int, comparator: Comparator[? >: T]): Unit
    @js("$arraysEqual($1, $2)")
    @jvm("rt $1:L $2:L rtcall arraysEquals(Ljava/lang/Object;Ljava/lang/Object;)Z")
    def equals[T](a: Array[T], b: Array[T]): Boolean
    @js("$arraysHash($1)")
    @jvm("rt $1:L rtcall arraysHashCode(Ljava/lang/Object;)I")
    def hashCode[T](a: Array[T]): Int
    @js("$arraysToString($1)")
    @jvm("rt $1:L rtcall arraysToString(Ljava/lang/Object;)Ljava/lang/String;")
    def toString[T](a: Array[T]): String
    @jvm("rt $1:L rtcall arraysAsList(Ljava/lang/Object;)Ljava/lang/Object; checkcast java/util/List")
    def asList[T](a: T*): List[T] =
      val out = new ArrayList[T]()
      a.foreach(x => out.add(x))
      out
    @jvm("rt $1:L $2:L rtcall arraysBinarySearch(Ljava/lang/Object;Ljava/lang/Object;)I")
    def binarySearch(a: Array[AnyRef], key: Any): Int =
      var lo = 0
      var hi = a.length - 1
      while lo <= hi do
        val mid = (lo + hi) >>> 1
        val c = a(mid).asInstanceOf[Comparable[Any]].compareTo(key)
        if c < 0 then lo = mid + 1
        else if c > 0 then hi = mid - 1
        else return mid
      -(lo + 1)
    @jvm("rt $1:L $2:L rtcall arraysBinarySearch(Ljava/lang/Object;Ljava/lang/Object;)I")
    def binarySearch(a: Array[scala.Long], key: scala.Long): Int =
      var lo = 0
      var hi = a.length - 1
      while lo <= hi do
        val mid = (lo + hi) >>> 1
        val v = a(mid)
        if v < key then lo = mid + 1
        else if v > key then hi = mid - 1
        else return mid
      -(lo + 1)
    @jvm("rt $1:L $2:L rtcall arraysBinarySearch(Ljava/lang/Object;Ljava/lang/Object;)I")
    def binarySearch(a: Array[Int], key: Int): Int =
      var lo = 0
      var hi = a.length - 1
      while lo <= hi do
        val mid = (lo + hi) >>> 1
        val v = a(mid)
        if v < key then lo = mid + 1
        else if v > key then hi = mid - 1
        else return mid
      -(lo + 1)

// `java.util.regex` over the JS engine's regular expressions: the members scala-java-time's
// parsers call, and those scala-library's `scala.util.matching.Regex` calls in link mode, where it
// runs over this package.
package java.util.regex:

  @jvmClass("java/util/regex/Pattern")
  final class Pattern private (val regex: String, val flags: Int):
    def pattern(): String = regex
    def matcher(input: CharSequence): Matcher = new Matcher(this, input.toString)
    override def toString: String = regex

  @jvmClass("java/util/regex/Pattern")
  object Pattern:
    val CASE_INSENSITIVE: Int = 2
    val COMMENTS: Int = 4
    val MULTILINE: Int = 8
    val DOTALL: Int = 32
    val UNICODE_CASE: Int = 64
    def compile(regex: String): Pattern = new Pattern(regex, 0)
    def compile(regex: String, flags: Int): Pattern =
      val prefix = (if (flags & CASE_INSENSITIVE) != 0 then "(?i)" else "") + (if (flags & MULTILINE) != 0 then "(?m)" else "") + (if (flags & DOTALL) != 0 then "(?s)" else "") + (if (flags & UNICODE_CASE) != 0 then "(?u)" else "") + (if (flags & COMMENTS) != 0 then "(?x)" else "")
      new Pattern(prefix + regex, flags)
    def matches(regex: String, input: CharSequence): Boolean = compile(regex).matcher(input).matches()
    // An embedded `\E` closes the quote, and is matched as an escaped `\` and an `E`.
    def quote(s: String): String =
      val sb = new java.lang.StringBuilder("\\Q")
      var at = 0
      var e = s.indexOf("\\E")
      while e >= 0 do
        sb.append(s.substring(at, e)).append("\\E\\\\E\\Q")
        at = e + 2
        e = s.indexOf("\\E", at)
      sb.append(s.substring(at)).append("\\E").toString

  // The engine's matches, with the start and end of each group ("d"): what `scala.util.matching.Regex`
  // runs on in the lean std, which link mode replaces with scala-library's.
  private[regex] object Engine:
    @js("$reExec($1, $2, $3)")
    def exec(regex: String, mode: String, source: String): Any
    @js("$reAll($1, $2, \"gd\")")
    def all(regex: String, source: String): Array[Any]
    @js("$reReplace($1, $2, $3, $4, $5)")
    def replace(regex: String, target: String, replacement: String, all: Boolean, wrap: Any => Any): String
    @js("($1[$2] ?? null)")
    def group(raw: Any, i: Int): String
    @js("$1.replace(/[\\\\$]/g, \"\\\\$&\")")
    def quote(s: String): String
    @js("($1.groups !== undefined && Object.hasOwn($1.groups, $2))")
    def hasGroup(raw: Any, name: String): Boolean
    @js("($1.groups[$2] ?? null)")
    def named(raw: Any, name: String): String
    @js("($1.length - 1)")
    def groupCount(raw: Any): Int
    // -1 for a group that took no part.
    @js("($1.indices[$2]?.[0] ?? -1)")
    def start(raw: Any, i: Int): Int
    @js("($1.indices[$2]?.[1] ?? -1)")
    def end(raw: Any, i: Int): Int

  @jvmClass("java/util/regex/Matcher")
  final class Matcher(pattern: Pattern, input: String):
    private var found: Any = null
    private var all: Array[Any] = null
    private var next = 0
    // The region the matcher looks at; positions are reported against the whole input.
    private var from = 0
    private var to = input.length
    private def region: String = if from == 0 && to == input.length then input else input.substring(from, to)
    private def isFound: Boolean = found != null
    def matches(): Boolean =
      found = Engine.exec(pattern.regex, "fd", region)
      isFound
    def lookingAt(): Boolean =
      found = Engine.exec(pattern.regex, "yd", region)
      isFound
    def find(): Boolean =
      if all == null then
        all = Engine.all(pattern.regex, region)
        next = 0
      found = if next < all.length then all(next) else null
      next += 1
      isFound
    private def current: Any =
      if !isFound then throw new IllegalStateException("No match found")
      found
    def group(): String = Engine.group(current, 0)
    def group(i: Int): String =
      val raw = current
      checkGroup(raw, i)
      Engine.group(raw, i)
    def group(name: String): String =
      if name == null then throw new NullPointerException("Group name")
      val raw = current
      if !Engine.hasGroup(raw, name) then throw new IllegalArgumentException("No group with name <" + name + ">")
      Engine.named(raw, name)
    def groupCount(): Int = if isFound then Engine.groupCount(found) else 0
    def start(): Int = start(0)
    def end(): Int = end(0)
    def start(i: Int): Int =
      val raw = current
      checkGroup(raw, i)
      val s = Engine.start(raw, i)
      if s < 0 then -1 else from + s
    def end(i: Int): Int =
      val raw = current
      checkGroup(raw, i)
      val e = Engine.end(raw, i)
      if e < 0 then -1 else from + e
    private def checkGroup(raw: Any, i: Int): Unit =
      if i < 0 || i > Engine.groupCount(raw) then throw new IndexOutOfBoundsException("No group " + i)
    def region(start: Int, end: Int): Matcher =
      if start < 0 || end > input.length || start > end then throw new IndexOutOfBoundsException("region")
      from = start
      to = end
      found = null
      all = null
      this
    def regionStart(): Int = from
    def regionEnd(): Int = to
    // What `appendReplacement` has copied of the input so far.
    private var appended = 0
    def appendReplacement(sb: java.lang.StringBuilder, replacement: String): Matcher =
      sb.append(input.substring(appended, start()))
      var i = 0
      while i < replacement.length do
        val c = replacement.charAt(i)
        if c == '\\' then
          i += 1
          if i >= replacement.length then throw new IllegalArgumentException("character to be escaped is missing")
          sb.append(replacement.charAt(i))
          i += 1
        else if c == '$' then
          i += 1
          if i >= replacement.length || !Character.isDigit(replacement.charAt(i)) then throw new IllegalArgumentException("Illegal group reference")
          var n = replacement.charAt(i) - '0'
          i += 1
          // A further digit belongs to the reference while the group it names exists.
          while i < replacement.length && Character.isDigit(replacement.charAt(i)) && n * 10 + (replacement.charAt(i) - '0') <= groupCount() do
            n = n * 10 + (replacement.charAt(i) - '0')
            i += 1
          val g = group(n)
          if g != null then sb.append(g)
        else
          sb.append(c)
          i += 1
      appended = end()
      this
    def appendTail(sb: java.lang.StringBuilder): java.lang.StringBuilder =
      sb.append(input.substring(appended))
    def replaceAll(replacement: String): String =
      reset()
      Engine.replace(pattern.regex, input, replacement, true, m => m)
    def replaceFirst(replacement: String): String =
      reset()
      Engine.replace(pattern.regex, input, replacement, false, m => m)
    def reset(): Matcher =
      found = null
      all = null
      appended = 0
      from = 0
      to = input.length
      this

  @jvmClass("java/util/regex/Matcher")
  object Matcher:
    @jvm("$1 invokestatic java/util/regex/Matcher.quoteReplacement(Ljava/lang/String;)Ljava/lang/String;")
    def quoteReplacement(s: String): String =
      if s == null then throw new NullPointerException()
      Engine.quote(s)
