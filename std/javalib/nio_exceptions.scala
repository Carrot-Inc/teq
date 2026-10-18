// The exceptions of the file system and of decoding (`java.nio.file`, `java.nio.charset`) and
// `UncheckedIOException`, under the JDK's names and hierarchy and with its messages, which the
// interpreter's natives throw (src/interp/files.rs); a file of their own, so that a program that
// reaches `IOException` enters these classes and not the file system's. On the JVM the classes
// are the JDK's.
package java.nio.file:

  // The message is the file, ` -> ` and the other file when there is one, `: ` and the reason
  // when there is one; the reason alone without a file.
  @jvmClass("java/nio/file/FileSystemException")
  class FileSystemException(file: String, other: String, reason: String) extends java.io.IOException(reason):
    def this(file: String) = this(file, null, null)
    def getFile: String = file
    def getOtherFile: String = other
    def getReason: String = reason
    override def getMessage: String =
      if file == null && other == null then reason
      else
        val sb = new java.lang.StringBuilder
        if file != null then sb.append(file)
        if other != null then sb.append(" -> ").append(other)
        if reason != null then sb.append(": ").append(reason)
        sb.toString

  @jvmClass("java/nio/file/NoSuchFileException")
  class NoSuchFileException(file: String, other: String, reason: String) extends FileSystemException(file, other, reason):
    def this(file: String) = this(file, null, null)

  @jvmClass("java/nio/file/FileAlreadyExistsException")
  class FileAlreadyExistsException(file: String, other: String, reason: String) extends FileSystemException(file, other, reason):
    def this(file: String) = this(file, null, null)

  @jvmClass("java/nio/file/AccessDeniedException")
  class AccessDeniedException(file: String, other: String, reason: String) extends FileSystemException(file, other, reason):
    def this(file: String) = this(file, null, null)

  @jvmClass("java/nio/file/NotDirectoryException")
  class NotDirectoryException(file: String) extends FileSystemException(file, null, null)

  @jvmClass("java/nio/file/DirectoryNotEmptyException")
  class DirectoryNotEmptyException(dir: String) extends FileSystemException(dir, null, null)

  @jvmClass("java/nio/file/FileSystemLoopException")
  class FileSystemLoopException(file: String) extends FileSystemException(file, null, null)

  @jvmClass("java/nio/file/NotLinkException")
  class NotLinkException(file: String, other: String, reason: String) extends FileSystemException(file, other, reason):
    def this(file: String) = this(file, null, null)

  @jvmClass("java/nio/file/AtomicMoveNotSupportedException")
  class AtomicMoveNotSupportedException(source: String, target: String, reason: String) extends FileSystemException(source, target, reason)

package java.nio.charset:

  @jvmClass("java/nio/charset/CharacterCodingException")
  class CharacterCodingException extends java.io.IOException(null)

  @jvmClass("java/nio/charset/MalformedInputException")
  class MalformedInputException(inputLength: Int) extends CharacterCodingException:
    def getInputLength: Int = inputLength
    override def getMessage: String = "Input length = " + inputLength

  @jvmClass("java/nio/charset/UnmappableCharacterException")
  class UnmappableCharacterException(inputLength: Int) extends CharacterCodingException:
    def getInputLength: Int = inputLength
    override def getMessage: String = "Input length = " + inputLength

package java.nio.channels:

  // What a closed channel's stream throws (`Files.newInputStream`, `newOutputStream`), no message.
  @jvmClass("java/nio/channels/ClosedChannelException")
  class ClosedChannelException extends java.io.IOException(null)

  @jvmClass("java/nio/channels/NonWritableChannelException")
  class NonWritableChannelException extends IllegalStateException(null: String)

  @jvmClass("java/nio/channels/NonReadableChannelException")
  class NonReadableChannelException extends IllegalStateException(null: String)

  @jvmClass("java/nio/channels/Channel")
  trait Channel extends java.io.Closeable:
    def isOpen(): Boolean

  @jvmClass("java/nio/channels/ReadableByteChannel")
  trait ReadableByteChannel extends Channel:
    def read(dst: java.nio.ByteBuffer): Int

  @jvmClass("java/nio/channels/WritableByteChannel")
  trait WritableByteChannel extends Channel:
    def write(src: java.nio.ByteBuffer): Int

  @jvmClass("java/nio/channels/ByteChannel")
  trait ByteChannel extends ReadableByteChannel, WritableByteChannel

  @jvmClass("java/nio/channels/SeekableByteChannel")
  trait SeekableByteChannel extends ByteChannel:
    def position(): Long
    def position(newPosition: Long): SeekableByteChannel
    def size(): Long
    def truncate(size: Long): SeekableByteChannel

  // The JDK's streams over a channel (`sun.nio.ch.ChannelOutputStream`, `ChannelInputStream`): a
  // write writes its whole range, a read gives what one read of the channel does, `available` what
  // a seekable channel has past its position; closing the stream closes the channel.
  object Channels:
    private def range(bs: Array[Byte], off: Int, len: Int): Unit =
      if off < 0 || len < 0 || off > bs.length - len then
        throw new IndexOutOfBoundsException(s"Range [$off, $off + $len) out of bounds for length ${bs.length}")
    def newOutputStream(ch: WritableByteChannel): java.io.OutputStream =
      java.util.Objects.requireNonNull(ch, "ch")
      new java.io.OutputStream:
        def write(b: Int): Unit = write(Array(b.toByte), 0, 1)
        override def write(bs: Array[Byte], off: Int, len: Int): Unit =
          range(bs, off, len)
          if len > 0 then
            val bb = java.nio.ByteBuffer.wrap(bs, off, len)
            while bb.remaining() > 0 do
              if ch.write(bb) <= 0 then throw new RuntimeException("no bytes written")
        override def close(): Unit = ch.close()
    def newInputStream(ch: ReadableByteChannel): java.io.InputStream =
      java.util.Objects.requireNonNull(ch, "ch")
      new java.io.InputStream:
        def read(): Int =
          val one = new Array[Byte](1)
          if read(one, 0, 1) == 1 then one(0) & 0xff else -1
        override def read(bs: Array[Byte], off: Int, len: Int): Int =
          range(bs, off, len)
          if len == 0 then 0 else ch.read(java.nio.ByteBuffer.wrap(bs, off, len))
        override def available(): Int = ch match
          case s: SeekableByteChannel => math.max(0L, s.size() - s.position()).min(Int.MaxValue.toLong).toInt
          case _ => 0
        override def close(): Unit = ch.close()

package java.io:

  // Its cause is required, a null one a `NullPointerException`.
  @jvmClass("java/io/UncheckedIOException")
  class UncheckedIOException(message: String, cause: IOException) extends RuntimeException(message, cause):
    if cause == null then throw new NullPointerException()
    def this(cause: IOException) = this(if cause == null then null else cause.toString, cause)
    override def getCause: IOException = cause
