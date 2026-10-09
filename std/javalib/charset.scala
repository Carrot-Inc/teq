// `java.nio.charset.Charset` and `StandardCharsets`, `String.getBytes` and the byte
// constructors of `String` for JavaScript, over UTF-8, ISO-8859-1, US-ASCII and UTF-16; on the
// JVM the classes and methods are the JDK's own (`@jvmClass`, `@jvm`).
package java.nio.charset:

  @javaDefined
  @jvmClass("java/nio/charset/Charset")
  final class Charset private[charset] (canonical: String) extends java.lang.Comparable[Charset], CharsetCoding:
    @jvm("invokevirtual java/nio/charset/Charset.name()Ljava/lang/String;")
    def name(): String = canonical
    @jvm("invokevirtual java/nio/charset/Charset.displayName()Ljava/lang/String;")
    def displayName(): String = canonical
    @jvm("invokevirtual java/nio/charset/Charset.compareTo(Ljava/nio/charset/Charset;)I")
    def compareTo(that: Charset): Int = canonical.compareToIgnoreCase(that.name())
    override def toString: String = canonical
    override def equals(that: Any): Boolean = that match
      case c: Charset => c.name() == canonical
      case _ => false
    override def hashCode: Int = canonical.hashCode

  @javaDefined
  @jvmClass("java/nio/charset/Charset")
  object Charset:
    private def canonicalName(name: String): String =
      val upper = name.toUpperCase.replace("_", "-")
      upper match
        case "UTF-8" | "UTF8" => "UTF-8"
        case "ISO-8859-1" | "ISO8859-1" | "LATIN1" | "ISO-LATIN-1" | "8859-1" => "ISO-8859-1"
        case "US-ASCII" | "ASCII" | "ISO646-US" => "US-ASCII"
        case "UTF-16" | "UTF16" => "UTF-16"
        case "UTF-16BE" | "UTF16BE" => "UTF-16BE"
        case "UTF-16LE" | "UTF16LE" => "UTF-16LE"
        case _ => null
    @jvm("invokestatic java/nio/charset/Charset.forName(Ljava/lang/String;)Ljava/nio/charset/Charset;")
    def forName(name: String): Charset =
      val canonical = canonicalName(name)
      if canonical == null then throw new UnsupportedCharsetException(name)
      StandardCharsets.named(canonical)
    @jvm("invokestatic java/nio/charset/Charset.isSupported(Ljava/lang/String;)Z")
    def isSupported(name: String): Boolean = canonicalName(name) != null
    @jvm("invokestatic java/nio/charset/Charset.defaultCharset()Ljava/nio/charset/Charset;")
    def defaultCharset(): Charset = StandardCharsets.UTF_8

  @javaDefined
  @jvmClass("java/nio/charset/StandardCharsets")
  object StandardCharsets:
    private val utf8 = new Charset("UTF-8")
    private val latin1 = new Charset("ISO-8859-1")
    private val ascii = new Charset("US-ASCII")
    private val utf16 = new Charset("UTF-16")
    private val utf16be = new Charset("UTF-16BE")
    private val utf16le = new Charset("UTF-16LE")
    private[charset] def named(canonical: String): Charset = canonical match
      case "ISO-8859-1" => latin1
      case "US-ASCII" => ascii
      case "UTF-16" => utf16
      case "UTF-16BE" => utf16be
      case "UTF-16LE" => utf16le
      case _ => utf8
    @jvm("getstatic java/nio/charset/StandardCharsets.UTF_8:Ljava/nio/charset/Charset;")
    def UTF_8: Charset = utf8
    @jvm("getstatic java/nio/charset/StandardCharsets.ISO_8859_1:Ljava/nio/charset/Charset;")
    def ISO_8859_1: Charset = latin1
    @jvm("getstatic java/nio/charset/StandardCharsets.US_ASCII:Ljava/nio/charset/Charset;")
    def US_ASCII: Charset = ascii
    @jvm("getstatic java/nio/charset/StandardCharsets.UTF_16:Ljava/nio/charset/Charset;")
    def UTF_16: Charset = utf16
    @jvm("getstatic java/nio/charset/StandardCharsets.UTF_16BE:Ljava/nio/charset/Charset;")
    def UTF_16BE: Charset = utf16be
    @jvm("getstatic java/nio/charset/StandardCharsets.UTF_16LE:Ljava/nio/charset/Charset;")
    def UTF_16LE: Charset = utf16le

  @javaDefined
  @jvmClass("java/nio/charset/UnsupportedCharsetException")
  class UnsupportedCharsetException(charsetName: String) extends IllegalArgumentException(charsetName):
    def getCharsetName(): String = charsetName

  /** The bytes of `s` in the charset, as `String.getBytes` gives them: a character the charset
    * lacks becomes `?`, UTF-16 starts with the byte order mark. */
  def encode(s: String, charset: Charset): Array[Byte] =
    val out = scala.collection.mutable.ArrayBuffer.empty[Byte]
    charset.name() match
      case "ISO-8859-1" | "US-ASCII" =>
        val limit = if charset.name() == "US-ASCII" then 128 else 256
        var i = 0
        while i < s.length do
          val c = s.charAt(i).toInt
          if c >= 0xd800 && c <= 0xdbff && i + 1 < s.length then i += 1
          out += (if c < limit then c.toByte else '?'.toByte)
          i += 1
      case "UTF-16" | "UTF-16BE" | "UTF-16LE" =>
        val bigEndian = charset.name() != "UTF-16LE"
        if charset.name() == "UTF-16" then
          out += 0xfe.toByte
          out += 0xff.toByte
        var i = 0
        while i < s.length do
          val c = s.charAt(i).toInt
          if bigEndian then
            out += (c >> 8).toByte
            out += c.toByte
          else
            out += c.toByte
            out += (c >> 8).toByte
          i += 1
      case _ =>
        var i = 0
        while i < s.length do
          val cp = s.codePointAt(i)
          if cp < 0x80 then out += cp.toByte
          else if cp < 0x800 then
            out += (0xc0 | (cp >> 6)).toByte
            out += (0x80 | (cp & 0x3f)).toByte
          else if cp < 0x10000 then
            if cp >= 0xd800 && cp <= 0xdfff then out += '?'.toByte
            else
              out += (0xe0 | (cp >> 12)).toByte
              out += (0x80 | ((cp >> 6) & 0x3f)).toByte
              out += (0x80 | (cp & 0x3f)).toByte
          else
            out += (0xf0 | (cp >> 18)).toByte
            out += (0x80 | ((cp >> 12) & 0x3f)).toByte
            out += (0x80 | ((cp >> 6) & 0x3f)).toByte
            out += (0x80 | (cp & 0x3f)).toByte
            i += 1
          i += 1
    out.toArray

  /** The string the bytes spell in the charset, a malformed sequence read as U+FFFD. */
  def decode(bytes: Array[Byte], offset: Int, length: Int, charset: Charset): String =
    val sb = new java.lang.StringBuilder()
    val end = offset + length
    charset.name() match
      case "ISO-8859-1" | "US-ASCII" =>
        val limit = if charset.name() == "US-ASCII" then 128 else 256
        var i = offset
        while i < end do
          val b = bytes(i) & 0xff
          sb.append(if b < limit then b.toChar else '�')
          i += 1
      case "UTF-16" | "UTF-16BE" | "UTF-16LE" =>
        var bigEndian = charset.name() != "UTF-16LE"
        var i = offset
        if charset.name() == "UTF-16" && length >= 2 then
          val b0 = bytes(offset) & 0xff
          val b1 = bytes(offset + 1) & 0xff
          if b0 == 0xfe && b1 == 0xff then i += 2
          else if b0 == 0xff && b1 == 0xfe then
            bigEndian = false
            i += 2
        while i + 1 < end do
          val hi = bytes(if bigEndian then i else i + 1) & 0xff
          val lo = bytes(if bigEndian then i + 1 else i) & 0xff
          sb.append(((hi << 8) | lo).toChar)
          i += 2
        if i < end then sb.append('�')
      case _ =>
        var i = offset
        while i < end do
          val b = bytes(i) & 0xff
          if b < 0x80 then
            sb.append(b.toChar)
            i += 1
          else
            val (need, init) =
              if (b & 0xe0) == 0xc0 then (1, b & 0x1f)
              else if (b & 0xf0) == 0xe0 then (2, b & 0x0f)
              else if (b & 0xf8) == 0xf0 then (3, b & 0x07)
              else (-1, 0)
            if need < 0 || i + need >= end then
              sb.append('�')
              i += 1
            else
              var cp = init
              var k = 1
              var ok = true
              while k <= need do
                val c = bytes(i + k) & 0xff
                if (c & 0xc0) != 0x80 then ok = false
                cp = (cp << 6) | (c & 0x3f)
                k += 1
              if ok then
                if cp >= 0x10000 then
                  sb.append((0xd800 + ((cp - 0x10000) >> 10)).toChar)
                  sb.append((0xdc00 + ((cp - 0x10000) & 0x3ff)).toChar)
                else sb.append(cp.toChar)
                i += need + 1
              else
                sb.append('�')
                i += 1
    sb.toString

package scala:

  extension (s: String)
    @jvm("$0 invokevirtual java/lang/String.getBytes()[B")
    def getBytes(): Array[Byte] = java.nio.charset.encode(s, java.nio.charset.StandardCharsets.UTF_8)
    @jvm("$0 $1 invokevirtual java/lang/String.getBytes(Ljava/lang/String;)[B")
    def getBytes(charsetName: String): Array[Byte] = java.nio.charset.encode(s, java.nio.charset.Charset.forName(charsetName))
    @jvm("$0 $1 invokevirtual java/lang/String.getBytes(Ljava/nio/charset/Charset;)[B")
    def getBytes(charset: java.nio.charset.Charset): Array[Byte] = java.nio.charset.encode(s, charset)
    @js("Array.from($0.substring($1, $2)).length")
    @jvm("invokevirtual java/lang/String.codePointCount(II)I")
    def codePointCount(beginIndex: Int, endIndex: Int): Int

