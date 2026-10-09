// Archives, checksums and digests under `teq interp` (src/interp/archive.rs): zip archives read
// whole (`ZipFile`) or as a stream (`ZipInputStream`, a local header at a time, an entry's data as it
// is read), written as the JDK's `ZipOutputStream` writes them (its headers, its data descriptors, its
// time stamps' extra fields, DEFLATE as zlib compresses, src/deflate.rs), gzip read and written,
// CRC-32, and the digests MD5, SHA-1 and SHA-256 with `HexFormat` to show them. `Inflater` decodes its
// input as it comes (`crate::zip::Inflating`); `Deflater` compresses its input whole at `finish`,
// which gives the bytes the JDK's give. JavaScript has none of these. On the JVM the classes are the
// JDK's.
package java.util.zip:

  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def crc32Update(crc: Int, b: Array[Byte], off: Int, len: Int): Int
  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def deflateAll(data: Array[Byte], level: Int, nowrap: Boolean): Array[Byte]
  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def inflaterNew(nowrap: Boolean): Int
  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def inflaterInput(inflater: Int, b: Array[Byte], off: Int, len: Int): Unit
  // What the input given so far decodes to, empty when it needs more.
  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def inflaterRun(inflater: Int): Array[Byte]
  // Whether it finished (1), the input it has not taken, the input it was given.
  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def inflaterInfo(inflater: Int): Array[Long]
  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def inflaterReset(inflater: Int): Unit
  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def inflaterEnd(inflater: Int): Unit
  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def zipOpen(path: String): Int
  // Per entry: its name, method, DOS time, CRC-32, compressed size, size, extra field and comment.
  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def zipEntries(zip: Int): Array[Any]
  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def zipRead(zip: Int, index: Int): Array[Byte]
  @js("$fail(\"UnsupportedOperationException\", \"zip archives are not available on JavaScript\")")
  def zipClose(zip: Int): Unit
  // The machine's offset from UTC, in seconds, at an instant in seconds.
  @js("$fail(\"UnsupportedOperationException\", \"the time zone is not available on JavaScript\")")
  def localOffset(epochSecond: Long): Int
  // The instant, in seconds, of a local date and time in the machine's zone.
  @js("$fail(\"UnsupportedOperationException\", \"the time zone is not available on JavaScript\")")
  def localInstant(year: Int, month: Int, day: Int, hour: Int, minute: Int, second: Int): Long
  @js("$fail(\"UnsupportedOperationException\", \"the time zone is not available on JavaScript\")")
  def zoneName(): String

  // `ZipUtils`: an instant's local date and time in the machine's zone, its DOS time (the year's
  // bits overflowing past 2107 as the JDK's int arithmetic does), the extended DOS time that keeps
  // the milliseconds a DOS time drops in its high half, 1980-01-01 for an earlier year; and back.
  private[zip] final class Local(val year: Int, val month: Int, val day: Int, val hour: Int, val minute: Int, val second: Int)
  private[zip] def local(millis: Long): Local =
    val seconds = Math.floorDiv(millis, 1000L)
    val at = seconds + localOffset(seconds)
    val days = Math.floorDiv(at, 86400L)
    val rest = Math.floorMod(at, 86400L).toInt
    // Days to the civil date (Howard Hinnant's algorithm).
    val z = days + 719468L
    val era = Math.floorDiv(z, 146097L)
    val doe = z - era * 146097L
    val yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365
    val doy = doe - (365 * yoe + yoe / 4 - yoe / 100)
    val mp = (5 * doy + 2) / 153
    val d = (doy - (153 * mp + 2) / 5 + 1).toInt
    val m = (if mp < 10 then mp + 3 else mp - 9).toInt
    val y = (yoe + era * 400 + (if m <= 2 then 1 else 0)).toInt
    new Local(y, m, d, rest / 3600, rest / 60 % 60, rest % 60)
  private[zip] final val DOSTIME_BEFORE_1980 = (1L << 21) | (1L << 16)
  private[zip] final val UPPER_DOSTIME_BOUND = 128L * 365 * 24 * 60 * 60 * 1000
  private[zip] final val UPPER_UNIXTIME_BOUND = 0x7fffffffL
  private[zip] final val WINDOWS_EPOCH_IN_MICROSECONDS = -11644473600000000L
  private[zip] final val WINDOWS_TIME_NOT_AVAILABLE = Long.MinValue
  private[zip] def javaToExtendedDosTime(time: Long): Long =
    val l = local(time)
    if l.year >= 1980 then
      ((((l.year - 1980) << 25) | (l.month << 21) | (l.day << 16) | (l.hour << 11) | (l.minute << 5) | (l.second >> 1)).toLong & 0xffffffffL) + ((time % 2000) << 32)
    else DOSTIME_BEFORE_1980
  private[zip] def dosToJavaTime(dos: Long): Long =
    localInstant(((dos >> 25) & 0x7f).toInt + 1980, ((dos >> 21) & 0x0f).toInt, ((dos >> 16) & 0x1f).toInt, ((dos >> 11) & 0x1f).toInt, ((dos >> 5) & 0x3f).toInt, ((dos << 1) & 0x3e).toInt) * 1000L
  private[zip] def extendedDosToJavaTime(xdostime: Long): Long = dosToJavaTime(xdostime & 0xffffffffL) + (xdostime >> 32)
  private[zip] def fileTimeToUnixTime(t: java.nio.file.attribute.FileTime): Long = t.to(java.util.concurrent.TimeUnit.SECONDS)
  private[zip] def fileTimeToWinTime(t: java.nio.file.attribute.FileTime): Long = (t.to(java.util.concurrent.TimeUnit.MICROSECONDS) - WINDOWS_EPOCH_IN_MICROSECONDS) * 10
  private[zip] def winTimeToFileTime(w: Long): java.nio.file.attribute.FileTime = java.nio.file.attribute.FileTime.from(w / 10 + WINDOWS_EPOCH_IN_MICROSECONDS, java.util.concurrent.TimeUnit.MICROSECONDS)
  private[zip] def get16(b: Array[Byte], off: Int): Int = (b(off) & 0xff) | ((b(off + 1) & 0xff) << 8)
  private[zip] def get32(b: Array[Byte], off: Int): Long = (get16(b, off) | (get16(b, off + 2).toLong << 16)) & 0xffffffffL
  private[zip] def get32S(b: Array[Byte], off: Int): Int = get16(b, off) | (get16(b, off + 2) << 16)
  private[zip] def get64S(b: Array[Byte], off: Int): Long = get32(b, off) | (get32(b, off + 4) << 32)
  private[zip] final val EXTID_ZIP64 = 0x0001
  private[zip] final val EXTID_NTFS = 0x000a
  private[zip] final val EXTID_EXTT = 0x5455

  @jvmClass("java/util/zip/Checksum")
  trait Checksum:
    def update(b: Int): Unit
    def update(b: Array[Byte], off: Int, len: Int): Unit
    def update(b: Array[Byte]): Unit = update(b, 0, b.length)
    def getValue(): Long
    def reset(): Unit

  @javaDefined
  @jvmClass("java/util/zip/CRC32")
  class CRC32 extends Checksum:
    private var crc = 0
    def update(b: Int): Unit = crc = crc32Update(crc, Array(b.toByte), 0, 1)
    def update(b: Array[Byte], off: Int, len: Int): Unit =
      if b == null then throw new NullPointerException()
      if off < 0 || len < 0 || off > b.length - len then throw new ArrayIndexOutOfBoundsException()
      crc = crc32Update(crc, b, off, len)
    def getValue(): Long = crc.toLong & 0xffffffffL
    def reset(): Unit = crc = 0

  // zlib's compression, the input taken whole: `finish`, then `deflate` gives the stream.
  @javaDefined
  @jvmClass("java/util/zip/Deflater")
  class Deflater(private var level: Int, nowrap: Boolean):
    if level != -1 && (level < 0 || level > 9) then throw new IllegalArgumentException("invalid compression level")
    def this(level: Int) = this(level, false)
    def this() = this(-1, false)
    private val input = new java.io.ByteArrayOutputStream()
    private var finishing = false
    private var output: Array[Byte] = null
    private var at = 0
    private var bytesRead = 0L
    def setInput(b: Array[Byte], off: Int, len: Int): Unit =
      input.write(b, off, len)
      bytesRead += len
    def setInput(b: Array[Byte]): Unit = setInput(b, 0, b.length)
    def setLevel(level: Int): Unit =
      if level != -1 && (level < 0 || level > 9) then throw new IllegalArgumentException("invalid compression level")
      this.level = level
    def setStrategy(strategy: Int): Unit = if strategy != 0 then throw new UnsupportedOperationException("a strategy other than the default")
    def needsInput(): Boolean = !finishing
    def finish(): Unit = finishing = true
    def finished(): Boolean = output != null && at >= output.length
    def deflate(b: Array[Byte]): Int = deflate(b, 0, b.length)
    def deflate(b: Array[Byte], off: Int, len: Int): Int =
      if !finishing then 0
      else
        if output == null then output = deflateAll(input.toByteArray(), level, nowrap)
        val n = Math.min(len, output.length - at)
        System.arraycopy(output, at, b, off, n)
        at += n
        n
    def getBytesRead(): Long = bytesRead
    def getBytesWritten(): Long = at.toLong
    def getTotalIn(): Int = bytesRead.toInt
    def getTotalOut(): Int = at
    def reset(): Unit =
      input.reset()
      finishing = false
      output = null
      at = 0
      bytesRead = 0L
    def end(): Unit = reset()

  @javaDefined
  @jvmClass("java/util/zip/Deflater")
  object Deflater:
    final val DEFLATED = 8
    final val NO_COMPRESSION = 0
    final val BEST_SPEED = 1
    final val BEST_COMPRESSION = 9
    final val DEFAULT_COMPRESSION = -1
    final val DEFAULT_STRATEGY = 0

  // zlib's decompression as the input comes: `setInput` gives it more, `inflate` what it decodes to
  // so far, `getRemaining` the input past the stream's end once it is finished.
  @javaDefined
  @jvmClass("java/util/zip/Inflater")
  class Inflater(nowrap: Boolean):
    def this() = this(false)
    private var id = inflaterNew(nowrap)
    private var out = new Array[Byte](0)
    private var at = 0
    private var written = 0L
    private def ensureOpen(): Unit = if id < 0 then throw new NullPointerException("Inflater has been closed")
    def setInput(b: Array[Byte], off: Int, len: Int): Unit =
      if b == null then throw new NullPointerException()
      if off < 0 || len < 0 || off > b.length - len then throw new ArrayIndexOutOfBoundsException()
      ensureOpen()
      inflaterInput(id, b, off, len)
    def setInput(b: Array[Byte]): Unit = setInput(b, 0, b.length)
    def inflate(b: Array[Byte], off: Int, len: Int): Int =
      if b == null then throw new NullPointerException()
      if off < 0 || len < 0 || off > b.length - len then throw new ArrayIndexOutOfBoundsException()
      ensureOpen()
      if at >= out.length then
        out = inflaterRun(id)
        at = 0
      val n = Math.min(len, out.length - at)
      System.arraycopy(out, at, b, off, n)
      at += n
      written += n
      n
    def inflate(b: Array[Byte]): Int = inflate(b, 0, b.length)
    private[zip] def hasPendingOutput: Boolean = at < out.length
    def finished(): Boolean =
      ensureOpen()
      !hasPendingOutput && inflaterInfo(id)(0) == 1L
    def needsInput(): Boolean =
      ensureOpen()
      !hasPendingOutput && inflaterInfo(id)(1) == 0L
    def needsDictionary(): Boolean = false
    def getRemaining(): Int =
      ensureOpen()
      inflaterInfo(id)(1).toInt
    def getBytesRead(): Long =
      ensureOpen()
      val info = inflaterInfo(id)
      info(2) - info(1)
    def getBytesWritten(): Long = written
    def getTotalIn(): Int = getBytesRead().toInt
    def getTotalOut(): Int = written.toInt
    def reset(): Unit =
      ensureOpen()
      inflaterReset(id)
      out = new Array[Byte](0)
      at = 0
      written = 0L
    def end(): Unit =
      if id >= 0 then
        inflaterEnd(id)
        id = -1

  // The JDK's: a stream's DEFLATE data read through an `Inflater`, `size` bytes of its source at a
  // time.
  @javaDefined
  @jvmClass("java/util/zip/InflaterInputStream")
  class InflaterInputStream(in: java.io.InputStream, protected val inf: Inflater, size: Int) extends java.io.FilterInputStream(in):
    if in == null || inf == null then throw new NullPointerException()
    if size <= 0 then throw new IllegalArgumentException("buffer size <= 0")
    def this(in: java.io.InputStream, inf: Inflater) = this(in, inf, 512)
    def this(in: java.io.InputStream) =
      this(in, new Inflater(), 512)
      usesDefaultInflater = true
    // An inflater of its own, which `close` ends.
    private[zip] var usesDefaultInflater = false
    protected val buf: Array[Byte] = new Array[Byte](size)
    protected var len: Int = 0
    private var closed = false
    private var reachEOF = false
    private def ensureOpen(): Unit = if closed then throw new java.io.IOException("Stream closed")
    override def read(): Int =
      val one = new Array[Byte](1)
      if read(one, 0, 1) == -1 then -1 else one(0) & 0xff
    override def read(b: Array[Byte], off: Int, len: Int): Int =
      ensureOpen()
      if b == null then throw new NullPointerException()
      if off < 0 || len < 0 || off > b.length - len then throw new IndexOutOfBoundsException()
      if len == 0 then 0
      else
        try
          var n = 0
          var done = false
          while !done do
            if inf.finished() || inf.needsDictionary() then
              reachEOF = true
              n = -1
              done = true
            else
              if inf.needsInput() && !inf.hasPendingOutput then fill()
              n = inf.inflate(b, off, len)
              done = n != 0
          n
        catch case e: DataFormatException => throw new ZipException(if e.getMessage != null then e.getMessage else "Invalid ZLIB data format")
    override def available(): Int =
      ensureOpen()
      if reachEOF then 0
      else if inf.finished() then
        reachEOF = true
        0
      else 1
    override def skip(n: Long): Long =
      if n < 0 then throw new IllegalArgumentException("negative skip length")
      ensureOpen()
      val max = Math.min(n, Int.MaxValue.toLong).toInt
      var total = 0
      val b = new Array[Byte](Math.min(max, 512))
      var more = true
      while more && total < max do
        val k = read(b, 0, Math.min(max - total, b.length))
        if k == -1 then
          reachEOF = true
          more = false
        else total += k
      total.toLong
    override def close(): Unit =
      if !closed then
        if usesDefaultInflater then inf.end()
        in.close()
        closed = true
    protected def fill(): Unit =
      ensureOpen()
      len = in.read(buf, 0, buf.length)
      if len == -1 then throw new java.io.EOFException("Unexpected end of ZLIB input stream")
      inf.setInput(buf, 0, len)

  @javaDefined
  @jvmClass("java/util/zip/DeflaterOutputStream")
  class DeflaterOutputStream(out: java.io.OutputStream, protected val `def`: Deflater) extends java.io.FilterOutputStream(out):
    def this(out: java.io.OutputStream) = this(out, new Deflater())
    private var closed = false
    override def write(b: Int): Unit = write(Array(b.toByte), 0, 1)
    override def write(b: Array[Byte], off: Int, len: Int): Unit =
      if `def`.finished() then throw new java.io.IOException("write beyond end of stream")
      `def`.setInput(b, off, len)
    def finish(): Unit =
      if !`def`.finished() then
        `def`.finish()
        val buf = new Array[Byte](8192)
        while !`def`.finished() do
          val n = `def`.deflate(buf, 0, buf.length)
          if n > 0 then out.write(buf, 0, n)
    override def flush(): Unit = out.flush()
    override def close(): Unit =
      if !closed then
        closed = true
        finish()
        out.close()

  @javaDefined
  @jvmClass("java/util/zip/GZIPOutputStream")
  class GZIPOutputStream(out: java.io.OutputStream) extends DeflaterOutputStream(out, new Deflater(-1, true)):
    private val crc = new CRC32()
    private var size = 0L
    out.write(Array[Byte](0x1f, 0x8b.toByte, 8, 0, 0, 0, 0, 0, 0, 0xff.toByte))
    override def write(b: Array[Byte], off: Int, len: Int): Unit =
      super.write(b, off, len)
      crc.update(b, off, len)
      size += len
    override def finish(): Unit =
      if !`def`.finished() then
        super.finish()
        val v = crc.getValue()
        out.write(Array[Byte](v.toByte, (v >> 8).toByte, (v >> 16).toByte, (v >> 24).toByte, size.toByte, (size >> 8).toByte, (size >> 16).toByte, (size >> 24).toByte))

  // The JDK's: a gzip member's header read when the stream is made, its data as it is read, its
  // trailer checked at its end, a member after it read on.
  @javaDefined
  @jvmClass("java/util/zip/GZIPInputStream")
  class GZIPInputStream(in: java.io.InputStream, size: Int) extends InflaterInputStream(in, new Inflater(true), size):
    def this(in: java.io.InputStream) = this(in, 512)
    usesDefaultInflater = true
    protected val crc = new CRC32()
    protected var eos = false
    private var closed = false
    try readHeader(in)
    catch
      case e: java.io.IOException =>
        inf.end()
        throw e
    private def ensureOpen(): Unit = if closed then throw new java.io.IOException("Stream closed")
    override def read(b: Array[Byte], off: Int, len: Int): Int =
      ensureOpen()
      if eos then -1
      else
        val n = super.read(b, off, len)
        if n == -1 then
          if readTrailer() then
            eos = true
            -1
          else read(b, off, len)
        else
          crc.update(b, off, n)
          n
    override def close(): Unit =
      if !closed then
        super.close()
        eos = true
        closed = true
    // The bytes the header took; its CRC (FHCRC) checked against the CRC-32 of what came before.
    private def readHeader(from: java.io.InputStream): Int =
      crc.reset()
      def byte(): Int =
        val b = from.read()
        if b == -1 then throw new java.io.EOFException()
        crc.update(b)
        b
      def ushort(): Int =
        val b = byte()
        (byte() << 8) | b
      if ushort() != 0x8b1f then throw new ZipException("Not in GZIP format")
      if byte() != 8 then throw new ZipException("Unsupported compression method")
      val flg = byte()
      var i = 0
      while i < 6 do
        byte()
        i += 1
      var n = 10
      if (flg & 4) == 4 then
        val m = ushort()
        var k = 0
        while k < m do
          byte()
          k += 1
        n += m + 2
      if (flg & 8) == 8 then
        n += 1
        while byte() != 0 do n += 1
      if (flg & 16) == 16 then
        n += 1
        while byte() != 0 do n += 1
      if (flg & 2) == 2 then
        val v = crc.getValue().toInt & 0xffff
        val b = from.read()
        val c = from.read()
        if b == -1 || c == -1 then throw new java.io.EOFException()
        if ((c << 8) | b) != v then throw new ZipException("Corrupt GZIP header")
        n += 2
      crc.reset()
      n
    // The trailer, from what the inflater left of its input and then the stream; true at the end,
    // false when another member follows (its data given to the inflater).
    private def readTrailer(): Boolean =
      val n = inf.getRemaining()
      val source = new java.io.InputStream:
        private var at = len - n
        def read(): Int =
          if at < len then
            at += 1
            buf(at - 1) & 0xff
          else in.read()
      def uint(): Long =
        var v = 0L
        var i = 0
        while i < 4 do
          val b = source.read()
          if b == -1 then throw new java.io.EOFException()
          v |= b.toLong << (8 * i)
          i += 1
        v
      if uint() != crc.getValue() || uint() != (inf.getBytesWritten() & 0xffffffffL) then throw new ZipException("Corrupt GZIP trailer")
      var m = 8
      try
        m += readHeader(source)
        inf.reset()
        if n > m then inf.setInput(buf, len - n + m, n - m)
        false
      catch case _: java.io.IOException => true

  // The JDK's entry: its DOS time with the milliseconds it drops (`xdostime`), the extended time
  // stamps of its extra field (`mtime`, `atime`, `ctime`), read from the field it is given and
  // written by `ZipOutputStream` as the JDK chooses them.
  @javaDefined
  @jvmClass("java/util/zip/ZipEntry")
  class ZipEntry(private var entryName: String):
    if entryName == null then throw new NullPointerException("name")
    if entryName.length > 0xffff then throw new IllegalArgumentException("entry name too long")
    private[zip] var xdostime = -1L
    private[zip] var mtime: java.nio.file.attribute.FileTime = null
    private[zip] var atime: java.nio.file.attribute.FileTime = null
    private[zip] var ctime: java.nio.file.attribute.FileTime = null
    private[zip] var crc = -1L
    private[zip] var size = -1L
    private[zip] var csize = -1L
    private[zip] var csizeSet = false
    private[zip] var method = -1
    private[zip] var flag = 0
    private[zip] var extra: Array[Byte] = null
    private[zip] var comment: String = null
    def this(e: ZipEntry) =
      this(e.getName)
      xdostime = e.xdostime
      mtime = e.mtime
      atime = e.atime
      ctime = e.ctime
      crc = e.crc
      size = e.size
      csize = e.csize
      csizeSet = e.csizeSet
      method = e.method
      flag = e.flag
      extra = e.extra
      comment = e.comment
    def getName: String = entryName
    def isDirectory: Boolean = entryName.endsWith("/")
    def setTime(time: Long): Unit =
      xdostime = javaToExtendedDosTime(time)
      if xdostime != DOSTIME_BEFORE_1980 && time <= UPPER_DOSTIME_BOUND then mtime = null
      else
        val year = local(time).year
        mtime = if year >= 1980 && year <= 2099 then null else java.nio.file.attribute.FileTime.fromMillis(time)
    def getTime: Long =
      if mtime != null then mtime.toMillis
      else if xdostime == -1 then -1L
      else extendedDosToJavaTime(xdostime)
    def setLastModifiedTime(time: java.nio.file.attribute.FileTime): ZipEntry =
      if time == null then throw new NullPointerException("lastModifiedTime")
      mtime = time
      xdostime = javaToExtendedDosTime(time.to(java.util.concurrent.TimeUnit.MILLISECONDS))
      this
    def getLastModifiedTime: java.nio.file.attribute.FileTime =
      if mtime != null then mtime
      else if xdostime == -1 then null
      else java.nio.file.attribute.FileTime.fromMillis(getTime)
    def setLastAccessTime(time: java.nio.file.attribute.FileTime): ZipEntry =
      if time == null then throw new NullPointerException("lastAccessTime")
      atime = time
      this
    def getLastAccessTime: java.nio.file.attribute.FileTime = atime
    def setCreationTime(time: java.nio.file.attribute.FileTime): ZipEntry =
      if time == null then throw new NullPointerException("creationTime")
      ctime = time
      this
    def getCreationTime: java.nio.file.attribute.FileTime = ctime
    def setSize(size: Long): Unit =
      if size < 0 then throw new IllegalArgumentException("invalid entry size")
      this.size = size
    def getSize: Long = size
    def getCompressedSize: Long = csize
    def setCompressedSize(csize: Long): Unit =
      this.csize = csize
      csizeSet = true
    def setCrc(crc: Long): Unit =
      if crc < 0 || crc > 0xffffffffL then throw new IllegalArgumentException("invalid entry crc-32")
      this.crc = crc
    def getCrc: Long = crc
    def setMethod(method: Int): Unit =
      if method != ZipEntry.STORED && method != ZipEntry.DEFLATED then throw new IllegalArgumentException("invalid compression method")
      this.method = method
    def getMethod: Int = method
    def setExtra(extra: Array[Byte]): Unit = setExtra0(extra, true)
    // The extra field and the time stamps it holds: an NTFS field's three times, an extended time
    // stamp's (in a central directory its modification time alone); the rest kept as it is.
    private[zip] def setExtra0(extra: Array[Byte], isLOC: Boolean): Unit =
      if extra != null then
        if extra.length + entryName.length + (if comment == null then 0 else comment.length) + 46 > 0xffff then
          throw new IllegalArgumentException("invalid extra field length")
        var off = 0
        val len = extra.length
        var more = true
        while more && off + 4 < len do
          val tag = get16(extra, off)
          val sz = get16(extra, off + 2)
          off += 4
          if off + sz > len then more = false
          else
            if tag == EXTID_NTFS then
              if sz >= 32 && get16(extra, off + 4) == 0x0001 && get16(extra, off + 6) == 24 then
                val pos = off + 4
                var w = get64S(extra, pos + 4)
                if w != WINDOWS_TIME_NOT_AVAILABLE then mtime = winTimeToFileTime(w)
                w = get64S(extra, pos + 12)
                if w != WINDOWS_TIME_NOT_AVAILABLE then atime = winTimeToFileTime(w)
                w = get64S(extra, pos + 20)
                if w != WINDOWS_TIME_NOT_AVAILABLE then ctime = winTimeToFileTime(w)
            else if tag == EXTID_EXTT then
              val flags = extra(off) & 0xff
              var sz0 = 1
              if (flags & 1) != 0 && sz0 + 4 <= sz then
                mtime = java.nio.file.attribute.FileTime.from(get32S(extra, off + sz0).toLong, java.util.concurrent.TimeUnit.SECONDS)
                sz0 += 4
              if (flags & 2) != 0 && sz0 + 4 <= sz then
                atime = java.nio.file.attribute.FileTime.from(get32S(extra, off + sz0).toLong, java.util.concurrent.TimeUnit.SECONDS)
                sz0 += 4
              if (flags & 4) != 0 && sz0 + 4 <= sz then
                ctime = java.nio.file.attribute.FileTime.from(get32S(extra, off + sz0).toLong, java.util.concurrent.TimeUnit.SECONDS)
                sz0 += 4
            off += sz
      this.extra = extra
    def getExtra: Array[Byte] = extra
    def setComment(comment: String): Unit = this.comment = comment
    def getComment: String = comment
    override def toString: String = entryName
    override def hashCode: Int = entryName.hashCode

  @javaDefined
  @jvmClass("java/util/zip/ZipEntry")
  object ZipEntry:
    final val STORED = 0
    final val DEFLATED = 8

  // An archive's entries, its central directory read when it is opened (each entry's extra field
  // read as the JDK reads a central directory's), an entry's data when asked for.
  @javaDefined
  @jvmClass("java/util/zip/ZipFile")
  class ZipFile(name: String):
    def this(file: java.io.File) = this(file.getPath)
    private var id = zipOpen(name)
    private val listed: Array[ZipEntry] =
      val raw = zipEntries(id)
      val out = new Array[ZipEntry](raw.length / 8)
      var i = 0
      while i < out.length do
        val e = new ZipEntry(raw(8 * i).asInstanceOf[String])
        e.method = raw(8 * i + 1).asInstanceOf[Long].toInt
        e.xdostime = raw(8 * i + 2).asInstanceOf[Long]
        e.crc = raw(8 * i + 3).asInstanceOf[Long]
        e.csize = raw(8 * i + 4).asInstanceOf[Long]
        e.size = raw(8 * i + 5).asInstanceOf[Long]
        e.comment = raw(8 * i + 7).asInstanceOf[String]
        val extra = raw(8 * i + 6).asInstanceOf[Array[Byte]]
        if extra != null then e.setExtra0(extra, false)
        out(i) = e
        i += 1
      out
    private def ensureOpen(): Unit = if id < 0 then throw new IllegalStateException("zip file closed")
    def getName: String = name
    def size(): Int =
      ensureOpen()
      listed.length
    def getEntry(name: String): ZipEntry =
      ensureOpen()
      listed.find(_.getName == name).orElse(listed.find(_.getName == name + "/")).orNull
    def entries(): java.util.Enumeration[? <: ZipEntry] =
      ensureOpen()
      val all = listed
      new java.util.Enumeration[ZipEntry]:
        private var i = 0
        def hasMoreElements(): Boolean = i < all.length
        def nextElement(): ZipEntry =
          if i >= all.length then throw new java.util.NoSuchElementException()
          i += 1
          all(i - 1)
    def stream(): java.util.stream.Stream[? <: ZipEntry] =
      ensureOpen()
      java.util.Arrays.asList(listed*).stream()
    def getInputStream(entry: ZipEntry): java.io.InputStream =
      ensureOpen()
      val i = listed.indexWhere(_.getName == entry.getName)
      if i < 0 then null else new java.io.ByteArrayInputStream(zipRead(id, i))
    def close(): Unit =
      if id >= 0 then
        zipClose(id)
        id = -1

  // The JDK's: an archive read from a stream one local header at a time (`getNextEntry` reads the
  // header alone), its data as it is read, a data descriptor's sizes and CRC-32 taken into the entry
  // when its data ends; what the inflater read past an entry's data is pushed back for the next.
  @javaDefined
  @jvmClass("java/util/zip/ZipInputStream")
  class ZipInputStream(source: java.io.InputStream) extends InflaterInputStream(new java.io.PushbackInputStream(source, 512), new Inflater(true), 512):
    usesDefaultInflater = true
    private val pushback = in.asInstanceOf[java.io.PushbackInputStream]
    private var entry: ZipEntry = null
    private var flag = 0
    private val crc = new CRC32()
    private var remaining = 0L
    private val tmpbuf = new Array[Byte](512)
    private var closed = false
    private var entryEOF = false
    private def ensureOpen(): Unit = if closed then throw new java.io.IOException("Stream closed")
    def getNextEntry(): ZipEntry =
      ensureOpen()
      if entry != null then closeEntry()
      crc.reset()
      inf.reset()
      entry = readLOC()
      if entry != null then
        if entry.method == ZipEntry.STORED then remaining = entry.size
        entryEOF = false
      entry
    def closeEntry(): Unit =
      ensureOpen()
      while read(tmpbuf, 0, tmpbuf.length) != -1 do ()
      entryEOF = true
    override def available(): Int =
      ensureOpen()
      if entryEOF then 0 else 1
    override def read(): Int =
      val one = new Array[Byte](1)
      if read(one, 0, 1) == -1 then -1 else one(0) & 0xff
    override def read(b: Array[Byte], off: Int, len: Int): Int =
      ensureOpen()
      if off < 0 || len < 0 || off > b.length - len then throw new IndexOutOfBoundsException()
      if len == 0 then 0
      else if entry == null then -1
      else if entry.method == ZipEntry.DEFLATED then
        val n = super.read(b, off, len)
        if n == -1 then
          readEnd(entry)
          entryEOF = true
          entry = null
        else crc.update(b, off, n)
        n
      else if remaining <= 0 then
        entryEOF = true
        entry = null
        -1
      else
        val want = Math.min(len.toLong, remaining).toInt
        val n = pushback.read(b, off, want)
        if n == -1 then throw new ZipException("unexpected EOF")
        crc.update(b, off, n)
        remaining -= n
        if remaining == 0 && entry.crc != crc.getValue() then
          throw new ZipException("invalid entry CRC (expected 0x" + java.lang.Long.toHexString(entry.crc) + " but got 0x" + java.lang.Long.toHexString(crc.getValue()) + ")")
        n
    override def skip(n: Long): Long =
      if n < 0 then throw new IllegalArgumentException("negative skip length")
      ensureOpen()
      val max = Math.min(n, Int.MaxValue.toLong).toInt
      var total = 0
      var more = true
      while more && total < max do
        val k = read(tmpbuf, 0, Math.min(max - total, tmpbuf.length))
        if k == -1 then
          entryEOF = true
          more = false
        else total += k
      total.toLong
    override def close(): Unit =
      if !closed then
        super.close()
        closed = true
    private def readFully(b: Array[Byte], off: Int, len: Int): Unit =
      var at = off
      while at < off + len do
        val n = pushback.read(b, at, off + len - at)
        if n == -1 then throw new java.io.EOFException()
        at += n
    // The next local header's entry, null at the central directory or the end.
    private def readLOC(): ZipEntry =
      try readFully(tmpbuf, 0, 30)
      catch case _: java.io.EOFException => return null
      if get32(tmpbuf, 0) != 0x04034b50L then null
      else
        flag = get16(tmpbuf, 6)
        val nameLen = get16(tmpbuf, 26)
        val nameBytes = new Array[Byte](nameLen)
        readFully(nameBytes, 0, nameLen)
        val e = new ZipEntry(new String(nameBytes, java.nio.charset.StandardCharsets.UTF_8))
        if (flag & 1) == 1 then throw new ZipException("encrypted ZIP entry not supported")
        e.method = get16(tmpbuf, 8)
        e.xdostime = get32(tmpbuf, 10)
        if (flag & 8) == 8 then
          if e.method != ZipEntry.DEFLATED then throw new ZipException("only DEFLATED entries can have EXT descriptor")
        else
          e.crc = get32(tmpbuf, 14)
          e.csize = get32(tmpbuf, 18)
          e.size = get32(tmpbuf, 22)
        val extraLen = get16(tmpbuf, 28)
        if extraLen > 0 then
          val extra = new Array[Byte](extraLen)
          readFully(extra, 0, extraLen)
          e.setExtra0(extra, true)
        e
    // The entry's end: the inflater's leftover input pushed back, the data descriptor read into
    // the entry, its sizes and CRC-32 checked against what was read.
    private def readEnd(e: ZipEntry): Unit =
      val n = inf.getRemaining()
      if n > 0 then pushback.unread(buf, len - n, n)
      if (flag & 8) == 8 then
        readFully(tmpbuf, 0, 16)
        val sig = get32(tmpbuf, 0)
        if sig != 0x08074b50L then
          e.crc = sig
          e.csize = get32(tmpbuf, 4)
          e.size = get32(tmpbuf, 8)
          pushback.unread(tmpbuf, 12, 4)
        else
          e.crc = get32(tmpbuf, 4)
          e.csize = get32(tmpbuf, 8)
          e.size = get32(tmpbuf, 12)
      if e.size != inf.getBytesWritten() then
        throw new ZipException("invalid entry size (expected " + e.size + " but got " + inf.getBytesWritten() + " bytes)")
      if e.csize != inf.getBytesRead() then
        throw new ZipException("invalid entry compressed size (expected " + e.csize + " but got " + inf.getBytesRead() + " bytes)")
      if e.crc != crc.getValue() then
        throw new ZipException("invalid entry CRC (expected 0x" + java.lang.Long.toHexString(e.crc) + " but got 0x" + java.lang.Long.toHexString(crc.getValue()) + ")")

  // An archive written as the JDK's writes it: per entry its local header (with a data descriptor
  // after a deflated entry's data), then the central directory and its end; names in UTF-8; an
  // entry's time stamps in an extended time stamp field, or an NTFS one for a time past 2038
  // (`UPPER_UNIXTIME_BOUND`), its own extra field after them less the fields those stand for.
  @javaDefined
  @jvmClass("java/util/zip/ZipOutputStream")
  class ZipOutputStream(out: java.io.OutputStream) extends java.io.FilterOutputStream(out):
    private final class XEntry(val entry: ZipEntry, val offset: Long)
    private val xentries = new java.util.ArrayList[XEntry]()
    private val names = new java.util.HashSet[String]()
    private val crc = new CRC32()
    private var current: XEntry = null
    private var written = 0L
    private var locoff = 0L
    private var method = ZipEntry.DEFLATED
    private var level = -1
    private var comment: Array[Byte] = null
    private var data: java.io.ByteArrayOutputStream = null
    private var finished = false
    private var closed = false

    private def ensureOpen(): Unit = if closed then throw new java.io.IOException("Stream closed")
    private def writeBytes(b: Array[Byte], off: Int, len: Int): Unit =
      out.write(b, off, len)
      written += len
    private def writeByte(v: Int): Unit = writeBytes(Array(v.toByte), 0, 1)
    private def writeShort(v: Int): Unit = writeBytes(Array((v & 0xff).toByte, ((v >>> 8) & 0xff).toByte), 0, 2)
    private def writeInt(v: Long): Unit = writeBytes(Array((v & 0xff).toByte, ((v >>> 8) & 0xff).toByte, ((v >>> 16) & 0xff).toByte, ((v >>> 24) & 0xff).toByte), 0, 4)
    private def writeLong(v: Long): Unit =
      writeInt(v & 0xffffffffL)
      writeInt(v >>> 32)
    private def version(e: ZipEntry): Int = if e.method == ZipEntry.DEFLATED then 20 else 10

    def setComment(comment: String): Unit = this.comment = if comment == null then null else comment.getBytes(java.nio.charset.StandardCharsets.UTF_8)
    def setMethod(method: Int): Unit =
      if method != ZipEntry.STORED && method != ZipEntry.DEFLATED then throw new IllegalArgumentException("invalid compression method")
      this.method = method
    def setLevel(level: Int): Unit =
      if level != -1 && (level < 0 || level > 9) then throw new IllegalArgumentException("invalid compression level")
      this.level = level

    def putNextEntry(e: ZipEntry): Unit =
      ensureOpen()
      if current != null then closeEntry()
      if e.xdostime == -1 then e.setTime(System.currentTimeMillis())
      if e.method == -1 then e.method = method
      e.flag = 0
      if e.method == ZipEntry.DEFLATED then
        if e.size == -1 || e.csize == -1 || e.crc == -1 || !e.csizeSet then e.flag = 8
      else
        if e.size == -1 then e.size = e.csize
        else if e.csize == -1 then e.csize = e.size
        else if e.size != e.csize then throw new ZipException("STORED entry where compressed != uncompressed size")
        if e.size == -1 || e.crc == -1 then throw new ZipException("STORED entry missing size, compressed size, or crc-32")
      if !names.add(e.getName) then throw new ZipException("duplicate entry: " + e.getName)
      e.flag |= 0x800
      current = new XEntry(e, written)
      xentries.add(current)
      writeLOC(current)
      data = new java.io.ByteArrayOutputStream()

    // The extended time stamps' flags and Unix times, and whether they need the NTFS field.
    private final class Stamps(e: ZipEntry):
      val flags: Int = (if e.mtime != null then 1 else 0) | (if e.atime != null then 2 else 0) | (if e.ctime != null then 4 else 0)
      val umtime: Long = if e.mtime == null then -1L else fileTimeToUnixTime(e.mtime)
      val uatime: Long = if e.atime == null then -1L else fileTimeToUnixTime(e.atime)
      val uctime: Long = if e.ctime == null then -1L else fileTimeToUnixTime(e.ctime)
      val ntfs: Boolean = umtime > UPPER_UNIXTIME_BOUND || uatime > UPPER_UNIXTIME_BOUND || uctime > UPPER_UNIXTIME_BOUND
      def count: Int = (if e.mtime != null then 4 else 0) + (if e.atime != null then 4 else 0) + (if e.ctime != null then 4 else 0)
      def writeNtfs(): Unit =
        writeShort(EXTID_NTFS)
        writeShort(32)
        writeInt(0)
        writeShort(0x0001)
        writeShort(24)
        writeLong(if e.mtime == null then WINDOWS_TIME_NOT_AVAILABLE else fileTimeToWinTime(e.mtime))
        writeLong(if e.atime == null then WINDOWS_TIME_NOT_AVAILABLE else fileTimeToWinTime(e.atime))
        writeLong(if e.ctime == null then WINDOWS_TIME_NOT_AVAILABLE else fileTimeToWinTime(e.ctime))

    // The extra field's length less its extended time stamp and ZIP64 fields, which are written
    // apart (`getExtraLen`), and the field itself so (`writeExtra`).
    private def extraLen(extra: Array[Byte]): Int =
      if extra == null then 0
      else
        var skipped = 0
        var off = 0
        var more = true
        while more && off + 4 <= extra.length do
          val tag = get16(extra, off)
          val sz = get16(extra, off + 2)
          if off + 4 + sz > extra.length then more = false
          else
            if tag == EXTID_EXTT || tag == EXTID_ZIP64 then skipped += sz + 4
            off += sz + 4
        extra.length - skipped
    private def writeExtra(extra: Array[Byte]): Unit =
      if extra != null then
        var off = 0
        var done = false
        while !done && off + 4 <= extra.length do
          val tag = get16(extra, off)
          val sz = get16(extra, off + 2)
          if off + 4 + sz > extra.length then
            writeBytes(extra, off, extra.length - off)
            done = true
          else
            if tag != EXTID_EXTT && tag != EXTID_ZIP64 then writeBytes(extra, off, sz + 4)
            off += sz + 4
        if !done && off < extra.length then writeBytes(extra, off, extra.length - off)

    private def writeLOC(x: XEntry): Unit =
      val e = x.entry
      var elen = extraLen(e.extra)
      writeInt(0x04034b50L)
      writeShort(version(e))
      writeShort(e.flag)
      writeShort(e.method)
      writeInt(e.xdostime)
      if (e.flag & 8) == 8 then
        writeInt(0)
        writeInt(0)
        writeInt(0)
      else
        writeInt(e.crc)
        writeInt(e.csize)
        writeInt(e.size)
      val name = e.getName.getBytes(java.nio.charset.StandardCharsets.UTF_8)
      writeShort(name.length)
      val stamps = new Stamps(e)
      if stamps.flags != 0 then elen += (if stamps.ntfs then 36 else stamps.count + 5)
      writeShort(elen)
      writeBytes(name, 0, name.length)
      if stamps.flags != 0 then
        if stamps.ntfs then stamps.writeNtfs()
        else
          writeShort(EXTID_EXTT)
          writeShort(stamps.count + 1)
          writeByte(stamps.flags)
          if e.mtime != null then writeInt(stamps.umtime)
          if e.atime != null then writeInt(stamps.uatime)
          if e.ctime != null then writeInt(stamps.uctime)
      writeExtra(e.extra)
      locoff = written

    override def write(b: Int): Unit = write(Array(b.toByte), 0, 1)
    override def write(b: Array[Byte], off: Int, len: Int): Unit =
      ensureOpen()
      if off < 0 || len < 0 || off > b.length - len then throw new IndexOutOfBoundsException()
      if current == null then throw new ZipException("no current ZIP entry")
      if current.entry.method == ZipEntry.DEFLATED then data.write(b, off, len)
      else
        written += len
        if written - locoff > current.entry.size then throw new ZipException("attempt to write past end of STORED entry")
        out.write(b, off, len)
      crc.update(b, off, len)

    def closeEntry(): Unit =
      ensureOpen()
      if current != null then
        val e = current.entry
        if e.method == ZipEntry.DEFLATED then
          val raw = data.toByteArray()
          val packed = deflateAll(raw, level, true)
          writeBytes(packed, 0, packed.length)
          if (e.flag & 8) == 0 then
            if e.size != raw.length then throw new ZipException("invalid entry size (expected " + e.size + " but got " + raw.length + " bytes)")
            if e.csize != packed.length then throw new ZipException("invalid entry compressed size (expected " + e.csize + " but got " + packed.length + " bytes)")
            if e.crc != crc.getValue() then throw new ZipException("invalid entry CRC-32 (expected 0x" + java.lang.Long.toHexString(e.crc) + " but got 0x" + java.lang.Long.toHexString(crc.getValue()) + ")")
          else
            e.size = raw.length.toLong
            e.csize = packed.length.toLong
            e.crc = crc.getValue()
            writeInt(0x08074b50L)
            writeInt(e.crc)
            writeInt(e.csize)
            writeInt(e.size)
        else
          if e.size != written - locoff then throw new ZipException("invalid entry size (expected " + e.size + " but got " + (written - locoff) + " bytes)")
          if e.crc != crc.getValue() then throw new ZipException("invalid entry crc-32 (expected 0x" + java.lang.Long.toHexString(e.crc) + " but got 0x" + java.lang.Long.toHexString(crc.getValue()) + ")")
        crc.reset()
        current = null
        data = null

    def finish(): Unit =
      ensureOpen()
      if !finished then
        if current != null then closeEntry()
        val off = written
        val it = xentries.iterator()
        while it.hasNext do writeCEN(it.next())
        writeEND(off, written - off)
        finished = true

    private def writeCEN(x: XEntry): Unit =
      val e = x.entry
      writeInt(0x02014b50L)
      writeShort(version(e))
      writeShort(version(e))
      writeShort(e.flag)
      writeShort(e.method)
      writeInt(e.xdostime)
      writeInt(e.crc)
      writeInt(e.csize)
      writeInt(e.size)
      val name = e.getName.getBytes(java.nio.charset.StandardCharsets.UTF_8)
      writeShort(name.length)
      var elen = extraLen(e.extra)
      val comment = if e.comment == null then null else e.comment.getBytes(java.nio.charset.StandardCharsets.UTF_8)
      val stamps = new Stamps(e)
      // The central directory's extended time stamp holds the modification time alone, its flags
      // the others'.
      if stamps.flags != 0 then elen += (if stamps.ntfs then 36 else if e.mtime != null then 9 else 5)
      if 46L + name.length + (if comment == null then 0 else comment.length) + elen > 0xffff then
        throw new ZipException("invalid CEN header (bad header size)")
      writeShort(elen)
      writeShort(if comment == null then 0 else Math.min(comment.length, 0xffff))
      writeShort(0)
      writeShort(0)
      writeInt(0)
      writeInt(x.offset)
      writeBytes(name, 0, name.length)
      if stamps.flags != 0 then
        if stamps.ntfs then stamps.writeNtfs()
        else
          writeShort(EXTID_EXTT)
          if e.mtime != null then
            writeShort(5)
            writeByte(stamps.flags)
            writeInt(stamps.umtime)
          else
            writeShort(1)
            writeByte(stamps.flags)
      writeExtra(e.extra)
      if comment != null then writeBytes(comment, 0, Math.min(comment.length, 0xffff))

    private def writeEND(off: Long, len: Long): Unit =
      writeInt(0x06054b50L)
      writeShort(0)
      writeShort(0)
      writeShort(xentries.size())
      writeShort(xentries.size())
      writeInt(len)
      writeInt(off)
      if comment != null then
        writeShort(comment.length)
        writeBytes(comment, 0, comment.length)
      else writeShort(0)

    override def close(): Unit =
      if !closed then
        finish()
        out.close()
        closed = true

package java.security:

  @js("$fail(\"UnsupportedOperationException\", \"digests are not available on JavaScript\")")
  def digestNew(algorithm: String): Int
  @js("$fail(\"UnsupportedOperationException\", \"digests are not available on JavaScript\")")
  def digestUpdate(digest: Int, b: Array[Byte], off: Int, len: Int): Unit
  @js("$fail(\"UnsupportedOperationException\", \"digests are not available on JavaScript\")")
  def digestFinish(digest: Int): Array[Byte]

  // MD5, SHA-1 and SHA-256, each `digest` resetting it for the next.
  @javaDefined
  @jvmClass("java/security/MessageDigest")
  class MessageDigest private (algorithm: String):
    private val id = digestNew(algorithm)
    def getAlgorithm: String = algorithm
    def getDigestLength: Int = algorithm.toUpperCase match
      case "MD5" => 16
      case "SHA-1" | "SHA1" | "SHA" => 20
      case _ => 32
    def update(input: Byte): Unit = digestUpdate(id, Array(input), 0, 1)
    def update(input: Array[Byte]): Unit = digestUpdate(id, input, 0, input.length)
    def update(input: Array[Byte], offset: Int, len: Int): Unit = digestUpdate(id, input, offset, len)
    def digest(): Array[Byte] = digestFinish(id)
    def digest(input: Array[Byte]): Array[Byte] =
      update(input)
      digest()
    def reset(): Unit = digestFinish(id)
    override def toString: String = algorithm + " Message Digest from SUN, <initialized>\n"

  @javaDefined
  @jvmClass("java/security/MessageDigest")
  object MessageDigest:
    def getInstance(algorithm: String): MessageDigest =
      if algorithm == null then throw new NullPointerException("null algorithm name")
      new MessageDigest(algorithm)
    def isEqual(a: Array[Byte], b: Array[Byte]): Boolean = java.util.Arrays.equals(a, b)

package java.util:

  // Bytes as hexadecimal digits and back, lower case unless asked otherwise.
  @javaDefined
  @jvmClass("java/util/HexFormat")
  final class HexFormat private (upper: Boolean, delim: String):
    private val digits = if upper then "0123456789ABCDEF" else "0123456789abcdef"
    def withUpperCase(): HexFormat = new HexFormat(true, delim)
    def withLowerCase(): HexFormat = new HexFormat(false, delim)
    def withDelimiter(delimiter: String): HexFormat = new HexFormat(upper, delimiter)
    def isUpperCase(): Boolean = upper
    def delimiter(): String = delim
    def formatHex(bytes: Array[Byte]): String = formatHex(bytes, 0, bytes.length)
    def formatHex(bytes: Array[Byte], fromIndex: Int, toIndex: Int): String =
      if fromIndex < 0 || fromIndex > toIndex || toIndex > bytes.length then throw new IndexOutOfBoundsException("Range [" + fromIndex + ", " + toIndex + ") out of bounds for length " + bytes.length)
      val sb = new java.lang.StringBuilder()
      var i = fromIndex
      while i < toIndex do
        if i > fromIndex then sb.append(delim)
        val b = bytes(i) & 0xff
        sb.append(digits.charAt(b >> 4)).append(digits.charAt(b & 15))
        i += 1
      sb.toString
    def toHexDigits(value: Byte): String =
      val b = value & 0xff
      "" + digits.charAt(b >> 4) + digits.charAt(b & 15)
    def parseHex(string: CharSequence): Array[Byte] =
      val s = string.toString
      val step = 2 + delim.length
      if s.isEmpty then new Array[Byte](0)
      else
        if (s.length + delim.length) % step != 0 then throw new IllegalArgumentException("extra or missing delimiters or values consisting of prefix, two hexadecimal digits, and suffix")
        val out = new Array[Byte]((s.length + delim.length) / step)
        var i = 0
        while i < out.length do
          val at = i * step
          if i > 0 && s.substring(at - delim.length, at) != delim then throw new IllegalArgumentException("found: \"" + s.substring(at - delim.length, at) + "\", expected: \"" + delim + "\", index: " + (at - delim.length) + " ch: " + s.charAt(at - delim.length).toInt)
          out(i) = ((HexFormat.fromHexDigit(s.charAt(at)) << 4) | HexFormat.fromHexDigit(s.charAt(at + 1))).toByte
          i += 1
        out
    override def toString: String = "uppercase: " + upper + ", delimiter: \"" + delim + "\", prefix: \"\", suffix: \"\""

  @javaDefined
  @jvmClass("java/util/HexFormat")
  object HexFormat:
    private val plain = new HexFormat(false, "")
    def of(): HexFormat = plain
    def ofDelimiter(delimiter: String): HexFormat = new HexFormat(false, delimiter)
    def isHexDigit(ch: Int): Boolean = (ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F')
    def fromHexDigit(ch: Int): Int =
      if ch >= '0' && ch <= '9' then ch - '0'
      else if ch >= 'a' && ch <= 'f' then ch - 'a' + 10
      else if ch >= 'A' && ch <= 'F' then ch - 'A' + 10
      else throw new NumberFormatException("not a hexadecimal digit: \"" + ch.toChar + "\" = " + ch)
