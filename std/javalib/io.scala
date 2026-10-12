// The files and streams of `java.io` under `teq interp` (src/interp/process.rs): `File` as a
// path's text, the streams of files, of a child's pipes, of sockets and of stdin, and the readers
// and writers over them, UTF-8 by default as the JDK's are now. A child's stdin is buffered as the
// JDK buffers it (what is written reaches the child at a flush); a file's and a socket's streams
// write through. A closed stream holds no descriptor, and a child's pipes are closed when it ends,
// what was left to read kept. JavaScript has no files. On the JVM the classes are the JDK's.
package java.io:

  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamRead(id: Int): Int
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamReadInto(id: Int, b: Array[Byte], off: Int, len: Int): Int
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamReadAll(id: Int, max: Int): Array[Byte]
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamReadText(id: Int, max: Int, report: Boolean): String
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamAvailable(id: Int): Int
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamSkip(id: Int, n: Long): Long
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamWrite(id: Int, b: Int): Unit
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamWriteFrom(id: Int, b: Array[Byte], off: Int, len: Int): Unit
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamWriteText(id: Int, text: String): Unit
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamFlush(id: Int): Unit
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamClose(id: Int): Unit
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamStdin(): Int
  // A file's stream, `plain` for `FileInputStream`'s exceptions rather than `Files`'.
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamOpenRead(path: String, plain: Boolean): Int
  // `flags`: CREATE 1, CREATE_NEW 2, APPEND 4, TRUNCATE_EXISTING 8, as `StandardOpenOption` names
  // them (the file is written, created only with CREATE or CREATE_NEW).
  @js("$fail(\"UnsupportedOperationException\", \"streams are not available on JavaScript\")")
  def streamOpenWrite(path: String, flags: Int, plain: Boolean): Int

  // How many bytes at the end of `bytes(from until to)` the JDK's UTF-8 decoder leaves for more
  // input (`decodeArrayLoop`'s underflow): the start of a sequence more bytes can still complete; 0
  // where they end at a sequence's end or inside one already malformed, which it answers at once (a
  // lead c0, c1 or f5 to ff, a continuation where a lead belongs, a three- or four-byte prefix whose
  // later byte is wrong). Under `teq interp` the native that the streams' own text reads share.
  private[java] def utf8Underflow(bytes: Array[Byte], from: Int, to: Int): Int =
    def cont(b: Int): Boolean = (b & 0xc0) == 0x80
    def at(back: Int, i: Int): Int = bytes(to - back + i) & 0xff
    var back = 1
    var result = -1
    while result < 0 && back <= 3 && to - back >= from do
      val b1 = at(back, 0)
      if !cont(b1) then
        result =
          if b1 >= 0xc2 && b1 <= 0xdf then (if back < 2 then back else 0)
          else if b1 >= 0xe0 && b1 <= 0xef then
            if back >= 3 then 0
            else if back > 1 && ((b1 == 0xe0 && (at(back, 1) & 0xe0) == 0x80) || !cont(at(back, 1))) then 0
            else back
          else if b1 >= 0xf0 && b1 <= 0xf4 then
            if back > 1 && ((b1 == 0xf0 && (at(back, 1) < 0x90 || at(back, 1) > 0xbf)) || (b1 == 0xf4 && (at(back, 1) & 0xf0) != 0x80) || !cont(at(back, 1))) then 0
            else if back > 2 && !cont(at(back, 2)) then 0
            else back
          else 0
      back += 1
    if result < 0 then 0 else result

  // A stream of the interpreter's, by its index among process.rs's streams.
  @javaDefined
  private[java] class NativeInputStream(id: Int) extends InputStream:
    private[java] def streamId: Int = id
    def read(): Int = streamRead(id)
    override def read(b: Array[Byte], off: Int, len: Int): Int =
      if b == null then throw new NullPointerException()
      streamReadInto(id, b, off, len)
    override def readAllBytes(): Array[Byte] = streamReadAll(id, Int.MaxValue)
    override def readNBytes(len: Int): Array[Byte] = streamReadAll(id, len)
    override def available(): Int = streamAvailable(id)
    // A negative `n` moves a file's stream back, as `FileInputStream` and a channel's stream do.
    override def skip(n: Long): Long = streamSkip(id, n)
    override def close(): Unit = streamClose(id)
    override def transferTo(out: OutputStream): Long =
      var n = 0L
      val chunk = new Array[Byte](8192)
      var k = read(chunk, 0, chunk.length)
      while k >= 0 do
        out.write(chunk, 0, k)
        n += k
        k = read(chunk, 0, chunk.length)
      n

  @javaDefined
  private[java] class NativeOutputStream(id: Int) extends OutputStream:
    private[java] def streamId: Int = id
    def write(b: Int): Unit = streamWrite(id, b)
    override def write(b: Array[Byte], off: Int, len: Int): Unit =
      if b == null then throw new NullPointerException()
      streamWriteFrom(id, b, off, len)
    override def flush(): Unit = streamFlush(id)
    override def close(): Unit = streamClose(id)

  // The JDK's: bytes pushed back (`unread`) read again first, at most `size` of them.
  @javaDefined
  @jvmClass("java/io/PushbackInputStream")
  class PushbackInputStream(in: InputStream, size: Int) extends FilterInputStream(in):
    if size <= 0 then throw new IllegalArgumentException("size <= 0")
    def this(in: InputStream) = this(in, 1)
    protected var buf: Array[Byte] = new Array[Byte](size)
    protected var pos: Int = size
    private def ensureOpen(): Unit = if buf == null then throw new IOException("Stream closed")
    override def read(): Int =
      ensureOpen()
      if pos < buf.length then
        pos += 1
        buf(pos - 1) & 0xff
      else super.read()
    override def read(b: Array[Byte], off: Int, len: Int): Int =
      ensureOpen()
      if b == null then throw new NullPointerException()
      if off < 0 || len < 0 || off > b.length - len then throw new IndexOutOfBoundsException()
      if len == 0 then 0
      else
        val avail = Math.min(buf.length - pos, len)
        if avail > 0 then
          System.arraycopy(buf, pos, b, off, avail)
          pos += avail
        if len - avail > 0 then
          val n = super.read(b, off + avail, len - avail)
          if n == -1 then (if avail == 0 then -1 else avail) else avail + n
        else avail
    def unread(b: Int): Unit =
      ensureOpen()
      if pos == 0 then throw new IOException("Push back buffer is full")
      pos -= 1
      buf(pos) = b.toByte
    def unread(b: Array[Byte], off: Int, len: Int): Unit =
      ensureOpen()
      if len > pos then throw new IOException("Push back buffer is full")
      pos -= len
      System.arraycopy(b, off, buf, pos, len)
    def unread(b: Array[Byte]): Unit = unread(b, 0, b.length)
    override def available(): Int =
      ensureOpen()
      val n = buf.length - pos
      val avail = super.available()
      if n > Int.MaxValue - avail then Int.MaxValue else n + avail
    override def skip(n: Long): Long =
      ensureOpen()
      if n <= 0 then 0L
      else
        var pskip = (buf.length - pos).toLong
        var left = n
        if pskip > 0 then
          if left < pskip then pskip = left
          pos += pskip.toInt
          left -= pskip
        if left > 0 then pskip += super.skip(left)
        pskip
    override def markSupported(): Boolean = false
    override def close(): Unit =
      if buf != null then
        buf = null
        in.close()

  // `System.in`: the interpreter's stdin, one stream.
  private[java] object Stdin:
    lazy val stream: InputStream = new NativeInputStream(streamStdin())

  @javaDefined
  @jvmClass("java/io/FileInputStream")
  class FileInputStream(file: File) extends NativeInputStream(streamOpenRead(file.getPath, true)):
    def this(name: String) = this(new File(name))

  @javaDefined
  @jvmClass("java/io/FileOutputStream")
  class FileOutputStream(file: File, append: Boolean) extends NativeOutputStream(streamOpenWrite(file.getPath, if append then 1 | 4 else 1 | 8, true)):
    def this(file: File) = this(file, false)
    def this(name: String) = this(new File(name), false)
    def this(name: String, append: Boolean) = this(new File(name), append)

  // A path's text, as the JDK's `File` keeps it: separators run together, none at the end; its
  // questions to the file system go through `java.nio.file`.
  @javaDefined
  @jvmClass("java/io/File")
  class File private (path: String, normalized: Boolean) extends Comparable[File]:
    private val text: String = if normalized then path else java.nio.file.pathNormalized(path)
    def this(pathname: String) =
      this(
        {
          if pathname == null then throw new NullPointerException()
          pathname
        },
        false
      )
    def this(parent: String, child: String) = this(if parent == null then child else if parent.isEmpty then File.separator + child else parent + File.separator + child, false)
    def this(parent: File, child: String) = this(if parent == null then child else parent.getPath + File.separator + child, false)
    def getPath: String = text
    def getName: String =
      val p = toPath.getFileName
      if p == null then "" else p.toString
    def getParent: String =
      val p = toPath.getParent
      if p == null then null else p.toString
    def getParentFile: File =
      val p = getParent
      if p == null then null else new File(p, true)
    def isAbsolute: Boolean = toPath.isAbsolute
    def getAbsolutePath: String = toPath.toAbsolutePath.toString
    def getAbsoluteFile: File = new File(getAbsolutePath, true)
    def getCanonicalPath: String = toPath.toAbsolutePath.normalize.toString
    def getCanonicalFile: File = new File(getCanonicalPath, true)
    def toPath: java.nio.file.Path = java.nio.file.Path.of(text)
    def exists: Boolean = java.nio.file.Files.exists(toPath)
    def isDirectory: Boolean = java.nio.file.Files.isDirectory(toPath)
    def isFile: Boolean = java.nio.file.Files.isRegularFile(toPath)
    def canExecute: Boolean = java.nio.file.Files.isExecutable(toPath)
    def length: Long = if isFile then java.nio.file.Files.size(toPath) else 0L
    def lastModified: Long =
      try java.nio.file.Files.getLastModifiedTime(toPath).toMillis
      catch case _: IOException => 0L
    def delete(): Boolean =
      try
        java.nio.file.Files.delete(toPath)
        true
      catch case _: IOException => false
    def mkdir(): Boolean =
      try
        java.nio.file.Files.createDirectory(toPath)
        true
      catch case _: IOException => false
    def mkdirs(): Boolean =
      if exists then false
      else
        try
          java.nio.file.Files.createDirectories(toPath)
          true
        catch case _: IOException => false
    // The entries' names in the system's order, null for no directory.
    def list(): Array[String] =
      try java.nio.file.fileEntries(text)
      catch case _: IOException => null
    def listFiles(): Array[File] =
      val names = list()
      if names == null then null else names.map(n => new File(this, n))
    def renameTo(dest: File): Boolean =
      try
        java.nio.file.Files.move(toPath, dest.toPath, java.nio.file.StandardCopyOption.REPLACE_EXISTING)
        true
      catch case _: IOException => false
    def compareTo(other: File): Int = if java.nio.file.windowsPaths then text.compareToIgnoreCase(other.getPath) else text.compareTo(other.getPath)
    override def equals(other: Any): Boolean = other match
      case f: File => compareTo(f) == 0
      case _ => false
    override def hashCode: Int = (if java.nio.file.windowsPaths then text.toLowerCase.hashCode else text.hashCode) ^ 1234321
    override def toString: String = text

  @javaDefined
  @jvmClass("java/io/File")
  object File:
    val separatorChar: Char = if java.nio.file.windowsPaths then '\\' else '/'
    val separator: String = separatorChar.toString
    val pathSeparatorChar: Char = if java.nio.file.windowsPaths then ';' else ':'
    val pathSeparator: String = pathSeparatorChar.toString

  // Bytes to characters: UTF-8 unless told otherwise, malformed input replaced, or reported as the
  // JDK's `MalformedInputException` by the reader `Files.newBufferedReader` makes (`reporting`; for
  // UTF-8 and US-ASCII). Over the interpreter's streams it decodes in the native, a chunk at a time.
  @javaDefined
  @jvmClass("java/io/InputStreamReader")
  class InputStreamReader(in: InputStream, cs: java.nio.charset.Charset) extends Reader:
    def this(in: InputStream) = this(in, java.nio.charset.StandardCharsets.UTF_8)
    def this(in: InputStream, charsetName: String) = this(in, java.nio.charset.Charset.forName(charsetName))
    private val utf8 = cs.name() == "UTF-8"
    private var report = false
    private[java] def reporting(): InputStreamReader =
      report = true
      this
    private var held: String = ""
    private var at = 0
    // Bytes of a sequence a chunk cut, read again in front of the next (UTF-8 over a stream of a
    // program's own).
    private var cut: Array[Byte] = new Array[Byte](0)

    // The next decoded text, null at the end.
    private[java] def chunk(): String =
      if at < held.length then
        val rest = held.substring(at)
        held = ""
        at = 0
        rest
      else
        in match
          case n: NativeInputStream if utf8 => streamReadText(n.streamId, 8192, report)
          case _ => decodeNext()

    private def decodeNext(): String =
      val bytes = new Array[Byte](8192)
      System.arraycopy(cut, 0, bytes, 0, cut.length)
      var n = cut.length
      var k = in.read(bytes, n, bytes.length - n)
      while k == 0 do k = in.read(bytes, n, bytes.length - n)
      if k < 0 && n == 0 then null
      else
        if k > 0 then n += k
        val keep = if utf8 && k >= 0 then utf8Underflow(bytes, 0, n) else 0
        cut = java.util.Arrays.copyOfRange(bytes, n - keep, n)
        if report && cs.name() == "US-ASCII" then
          var i = 0
          while i < n - keep do
            if bytes(i) < 0 then throw new java.nio.charset.MalformedInputException(1)
            i += 1
        new String(bytes, 0, n - keep, cs)

    def read(): Int =
      if at >= held.length then
        val c = chunk()
        if c == null then return -1
        held = c
        at = 0
      if held.isEmpty then read()
      else
        at += 1
        held.charAt(at - 1).toInt
    def read(cbuf: Array[Char], off: Int, len: Int): Int =
      if len == 0 then 0
      else
        if at >= held.length then
          var c = chunk()
          while c != null && c.isEmpty do c = chunk()
          if c == null then return -1
          held = c
          at = 0
        val n = if len < held.length - at then len else held.length - at
        held.getChars(at, at + n, cbuf, off)
        at += n
        n
    def ready(): Boolean = at < held.length || in.available() > 0
    def close(): Unit = in.close()
    def getEncoding: String = if utf8 then "UTF8" else cs.name()

  // Lines and characters over a reader. A line ends at `\n`, `\r` or `\r\n`, the `\n` of a
  // `\r\n` taken with the next read, as the JDK's.
  @javaDefined
  @jvmClass("java/io/BufferedReader")
  class BufferedReader(in: Reader, size: Int) extends Reader:
    if size <= 0 then throw new IllegalArgumentException("Buffer size <= 0")
    def this(in: Reader) = this(in, 8192)
    private var buf: String = ""
    private var pos = 0
    private var skipLF = false
    private var closed = false
    // Where the next `\r` at or after `pos` is, -1 for none in `buf`, -2 when unknown.
    private var cr = -2

    // More text behind the buffer: false at the end.
    private def more(): Boolean =
      val c = in match
        case r: InputStreamReader => r.chunk()
        case r: BufferedReader => r.readRest()
        case _ =>
          val a = new Array[Char](size)
          val n = in.read(a, 0, a.length)
          if n < 0 then null else new String(a, 0, n)
      if c == null then false
      else
        buf = if pos >= buf.length then c else buf.substring(pos) + c
        pos = 0
        cr = -2
        true

    private[java] def readRest(): String =
      if pos < buf.length then
        val rest = buf.substring(pos)
        buf = ""
        pos = 0
        cr = -2
        rest
      else if more() then readRest()
      else null

    private def ensureOpen(): Unit = if closed then throw new IOException("Stream closed")

    def readLine(): String =
      ensureOpen()
      if skipLF then
        if pos >= buf.length then more()
        if pos < buf.length && buf.charAt(pos) == '\n' then pos += 1
        skipLF = false
      var line: String = null
      var done = false
      while !done do
        val nl = buf.indexOf('\n', pos)
        if cr == -2 || (cr >= 0 && cr < pos) then cr = buf.indexOf('\r', pos)
        val end = if cr >= 0 && (nl < 0 || cr < nl) then cr else nl
        if end >= 0 then
          line = buf.substring(pos, end)
          pos = end + 1
          if end == cr then
            cr = -2
            if pos < buf.length then
              if buf.charAt(pos) == '\n' then pos += 1
            else skipLF = true
          done = true
        else if !more() then
          if pos < buf.length then
            line = buf.substring(pos)
            pos = buf.length
          done = true
      line

    def read(): Int =
      ensureOpen()
      if pos >= buf.length && !more() then -1
      else
        val c = buf.charAt(pos)
        pos += 1
        if skipLF then
          skipLF = false
          if c == '\n' then return read()
        c.toInt
    def read(cbuf: Array[Char], off: Int, len: Int): Int =
      ensureOpen()
      if len == 0 then 0
      else if pos >= buf.length && !more() then -1
      else
        if skipLF then
          skipLF = false
          if buf.charAt(pos) == '\n' then
            pos += 1
            if pos >= buf.length && !more() then return -1
        val n = if len < buf.length - pos then len else buf.length - pos
        buf.getChars(pos, pos + n, cbuf, off)
        pos += n
        n
    def ready(): Boolean =
      ensureOpen()
      pos < buf.length || in.ready()
    def lines(): java.util.stream.Stream[String] =
      val list = new java.util.ArrayList[String]()
      var line = readLine()
      while line != null do
        list.add(line)
        line = readLine()
      list.stream()
    def close(): Unit =
      if !closed then
        closed = true
        in.close()

  // Characters to bytes, UTF-8 unless told otherwise.
  @javaDefined
  @jvmClass("java/io/OutputStreamWriter")
  class OutputStreamWriter(out: OutputStream, cs: java.nio.charset.Charset) extends Writer:
    def this(out: OutputStream) = this(out, java.nio.charset.StandardCharsets.UTF_8)
    def this(out: OutputStream, charsetName: String) = this(out, java.nio.charset.Charset.forName(charsetName))
    private val utf8 = cs.name() == "UTF-8"
    def write(cbuf: Array[Char], off: Int, len: Int): Unit = write(new String(cbuf, off, len))
    override def write(str: String): Unit =
      out match
        case n: NativeOutputStream if utf8 => streamWriteText(n.streamId, str)
        case _ => out.write(str.getBytes(cs))
    override def write(c: Int): Unit = write(c.toChar.toString)
    def flush(): Unit = out.flush()
    def close(): Unit = out.close()
    def getEncoding: String = if utf8 then "UTF8" else cs.name()

  @javaDefined
  @jvmClass("java/io/BufferedWriter")
  class BufferedWriter(out: Writer, size: Int) extends Writer:
    if size <= 0 then throw new IllegalArgumentException("Buffer size <= 0")
    def this(out: Writer) = this(out, 8192)
    private val buf = new java.lang.StringBuilder()
    private var closed = false
    private def ensureOpen(): Unit = if closed then throw new IOException("Stream closed")
    private def flushBuffer(): Unit =
      if buf.length > 0 then
        out.write(buf.toString)
        buf.setLength(0)
    def write(cbuf: Array[Char], off: Int, len: Int): Unit = write(new String(cbuf, off, len))
    override def write(str: String): Unit =
      ensureOpen()
      buf.append(str)
      if buf.length >= size then flushBuffer()
    override def write(c: Int): Unit = write(c.toChar.toString)
    def newLine(): Unit = write(System.lineSeparator())
    def flush(): Unit =
      ensureOpen()
      flushBuffer()
      out.flush()
    def close(): Unit =
      if !closed then
        flushBuffer()
        closed = true
        out.close()
