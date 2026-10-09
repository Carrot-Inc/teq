package java.nio:

  // A char buffer over an array, which a library builds its strings in (http4s' `UriCoding`
  // percent-encodes into one at compile time); on the JVM the JDK's class, whose members the
  // templates call.
  @javaDefined
  @jvmClass("java/nio/CharBuffer")
  // Over an array, or read-only over a character sequence it reads as it stands (`seq`), whose
  // length when wrapped is the capacity.
  final class CharBuffer private (chars: Array[Char], seq: java.lang.CharSequence, cap: Int, private var pos: Int, private var lim: Int, readOnly: Boolean) extends java.lang.CharSequence:
    private def at(i: Int): Char = if seq == null then chars(i) else seq.charAt(i)
    @jvm("invokevirtual java/nio/CharBuffer.length()I")
    def length: Int = lim - pos
    @jvm("invokevirtual java/nio/CharBuffer.charAt(I)C")
    def charAt(index: Int): Char =
      if index < 0 || index >= lim - pos then throw new IndexOutOfBoundsException(index.toString)
      at(pos + index)
    @jvm("invokevirtual java/nio/CharBuffer.subSequence(II)Ljava/nio/CharBuffer;")
    def subSequence(start: Int, end: Int): CharBuffer =
      if start < 0 || end > lim - pos || start > end then throw new IndexOutOfBoundsException()
      new CharBuffer(chars, seq, cap, pos + start, pos + end, readOnly)
    @jvm("invokevirtual java/nio/CharBuffer.isReadOnly()Z")
    def isReadOnly(): Boolean = readOnly
    @jvm("invokevirtual java/nio/CharBuffer.position()I")
    def position(): Int = pos
    @jvm("invokevirtual java/nio/CharBuffer.position(I)Ljava/nio/CharBuffer;")
    def position(p: Int): CharBuffer =
      if p < 0 || p > lim then throw new IllegalArgumentException("position " + p)
      pos = p
      this
    @jvm("invokevirtual java/nio/CharBuffer.limit()I")
    def limit(): Int = lim
    @jvm("invokevirtual java/nio/CharBuffer.limit(I)Ljava/nio/CharBuffer;")
    def limit(l: Int): CharBuffer =
      if l < 0 || l > capacity() then throw new IllegalArgumentException("limit " + l)
      lim = l
      if pos > l then pos = l
      this
    @jvm("invokevirtual java/nio/CharBuffer.capacity()I")
    def capacity(): Int = cap
    @jvm("invokevirtual java/nio/CharBuffer.remaining()I")
    def remaining(): Int = lim - pos
    @jvm("invokevirtual java/nio/CharBuffer.hasRemaining()Z")
    def hasRemaining(): Boolean = pos < lim
    @jvm("invokevirtual java/nio/CharBuffer.get()C")
    def get(): Char =
      if pos >= lim then throw new BufferUnderflowException()
      pos += 1
      at(pos - 1)
    @jvm("invokevirtual java/nio/CharBuffer.put(C)Ljava/nio/CharBuffer;")
    def put(c: Char): CharBuffer =
      if readOnly then throw new ReadOnlyBufferException()
      if pos >= lim then throw new BufferOverflowException()
      chars(pos) = c
      pos += 1
      this
    @jvm("invokevirtual java/nio/CharBuffer.flip()Ljava/nio/CharBuffer;")
    def flip(): CharBuffer =
      lim = pos
      pos = 0
      this
    @jvm("invokevirtual java/nio/CharBuffer.clear()Ljava/nio/CharBuffer;")
    def clear(): CharBuffer =
      pos = 0
      lim = capacity()
      this
    @jvm("invokevirtual java/nio/CharBuffer.toString()Ljava/lang/String;")
    override def toString: String =
      val sb = new java.lang.StringBuilder(lim - pos)
      var i = pos
      while i < lim do
        sb.append(at(i))
        i += 1
      sb.toString

  @jvmClass("java/nio/CharBuffer")
  object CharBuffer:
    @jvm("invokestatic java/nio/CharBuffer.wrap([CII)Ljava/nio/CharBuffer;")
    def wrap(array: Array[Char], offset: Int, length: Int): CharBuffer =
      if offset < 0 || length < 0 || offset > array.length - length then throw new IndexOutOfBoundsException()
      new CharBuffer(array, null, array.length, offset, offset + length, false)
    @jvm("invokestatic java/nio/CharBuffer.wrap([C)Ljava/nio/CharBuffer;")
    def wrap(array: Array[Char]): CharBuffer = new CharBuffer(array, null, array.length, 0, array.length, false)
    @jvm("invokestatic java/nio/CharBuffer.wrap(Ljava/lang/CharSequence;)Ljava/nio/CharBuffer;")
    def wrap(text: java.lang.CharSequence): CharBuffer =
      val n = text.length
      new CharBuffer(null, text, n, 0, n, true)
    @jvm("invokestatic java/nio/CharBuffer.allocate(I)Ljava/nio/CharBuffer;")
    def allocate(capacity: Int): CharBuffer = new CharBuffer(new Array[Char](capacity), null, capacity, 0, capacity, false)

  @jvmClass("java/nio/BufferUnderflowException")
  class BufferUnderflowException() extends RuntimeException()
  @jvmClass("java/nio/BufferOverflowException")
  class BufferOverflowException() extends RuntimeException()
  @jvmClass("java/nio/ReadOnlyBufferException")
  class ReadOnlyBufferException() extends UnsupportedOperationException()

package java.nio.charset:

  // `Charset`'s members over the std's buffers, on JavaScript, in the interpreter and on the
  // JVM, whose platform layer keeps them beside scala-library's bytecode.
  trait CharsetCoding:
    this: Charset =>
    @jvm("invokevirtual java/nio/charset/Charset.encode(Ljava/lang/String;)Ljava/nio/ByteBuffer;")
    def encode(s: String): java.nio.ByteBuffer = java.nio.ByteBuffer.wrap(java.nio.charset.encode(s, this))
    @jvm("invokevirtual java/nio/charset/Charset.encode(Ljava/nio/CharBuffer;)Ljava/nio/ByteBuffer;")
    def encode(chars: java.nio.CharBuffer): java.nio.ByteBuffer =
      val s = chars.toString
      chars.position(chars.limit())
      encode(s)
    @jvm("invokevirtual java/nio/charset/Charset.decode(Ljava/nio/ByteBuffer;)Ljava/nio/CharBuffer;")
    def decode(bytes: java.nio.ByteBuffer): java.nio.CharBuffer =
      val n = bytes.remaining()
      val chunk = new Array[Byte](n)
      bytes.get(chunk)
      java.nio.CharBuffer.wrap(java.nio.charset.decode(chunk, 0, n, this).toCharArray)
