// `java.util.UUID`, `Base64`, `Locale`, `TimeZone`, `Date`, `BitSet`, `java.util.concurrent.TimeUnit`
// and `ConcurrentHashMap` for JavaScript, written from the JDK's documented behaviour; on the JVM
// the classes are the JDK's own (`@jvmClass`), whose std bodies a macro runs in the interpreter.
package java.util:

  // Bits over growable words of 32; cats-parse's character classes (http4s' `uri"..."`).
  @jvmClass("java/util/BitSet")
  final class BitSet(nbits: Int):
    def this() = this(64)
    if nbits < 0 then throw new NegativeArraySizeException("nbits < 0: " + nbits)
    private var words = new Array[Int]((nbits + 31) / 32)
    private def ensure(wordIndex: Int): Unit =
      if wordIndex >= words.length then
        val grown = new Array[Int](math.max(wordIndex + 1, 2 * words.length))
        System.arraycopy(words, 0, grown, 0, words.length)
        words = grown
    private def checkIndex(i: Int): Unit =
      if i < 0 then throw new IndexOutOfBoundsException("bitIndex < 0: " + i)
    private def checkRange(from: Int, to: Int): Unit =
      if from < 0 then throw new IndexOutOfBoundsException("fromIndex < 0: " + from)
      if to < from then throw new IndexOutOfBoundsException("fromIndex: " + from + " > toIndex: " + to)
    def get(i: Int): Boolean =
      checkIndex(i)
      i / 32 < words.length && (words(i / 32) & (1 << (i % 32))) != 0
    def set(i: Int): Unit =
      checkIndex(i)
      ensure(i / 32)
      words(i / 32) = words(i / 32) | (1 << (i % 32))
    def set(i: Int, value: Boolean): Unit = if value then set(i) else clear(i)
    def set(from: Int, to: Int): Unit =
      checkRange(from, to)
      var i = from
      while i < to do
        set(i)
        i += 1
    def clear(i: Int): Unit =
      checkIndex(i)
      if i / 32 < words.length then words(i / 32) = words(i / 32) & ~(1 << (i % 32))
    def clear(): Unit =
      var w = 0
      while w < words.length do
        words(w) = 0
        w += 1
    def flip(i: Int): Unit = if get(i) then clear(i) else set(i)
    def flip(from: Int, to: Int): Unit =
      checkRange(from, to)
      var i = from
      while i < to do
        flip(i)
        i += 1
    def cardinality(): Int =
      var n = 0
      var w = 0
      while w < words.length do
        n += java.lang.Integer.bitCount(words(w))
        w += 1
      n
    def nextSetBit(from: Int): Int =
      if from < 0 then throw new IndexOutOfBoundsException("fromIndex < 0: " + from)
      var i = from
      val end = words.length * 32
      while i < end && !get(i) do i += 1
      if i < end then i else -1
    def nextClearBit(from: Int): Int =
      if from < 0 then throw new IndexOutOfBoundsException("fromIndex < 0: " + from)
      var i = from
      while get(i) do i += 1
      i
    def length(): Int =
      var i = words.length * 32 - 1
      while i >= 0 && !get(i) do i -= 1
      i + 1
    def size(): Int = words.length * 32
    def isEmpty(): Boolean = length() == 0
    private def combine(other: BitSet, op: (Int, Int) => Int): Unit =
      ensure(other.words.length - 1)
      var w = 0
      while w < words.length do
        words(w) = op(words(w), if w < other.words.length then other.words(w) else 0)
        w += 1
    def and(other: BitSet): Unit = combine(other, _ & _)
    def or(other: BitSet): Unit = combine(other, _ | _)
    def xor(other: BitSet): Unit = combine(other, _ ^ _)
    def andNot(other: BitSet): Unit = combine(other, (a, b) => a & ~b)
    override def clone(): AnyRef =
      val copy = new BitSet(words.length * 32)
      System.arraycopy(words, 0, copy.words, 0, words.length)
      copy
    override def equals(that: Any): Boolean = that match
      case b: BitSet =>
        val n = math.max(length(), b.length())
        var i = 0
        while i < n && get(i) == b.get(i) do i += 1
        i == n
      case _ => false
    override def hashCode: Int =
      var h = 1234
      var i = 0
      while i < length() do
        if get(i) then h = h ^ (i * 31 + 7)
        i += 1
      h
    override def toString: String =
      val out = new java.lang.StringBuilder("{")
      var i = nextSetBit(0)
      var first = true
      while i >= 0 do
        if !first then out.append(", ")
        out.append(i)
        first = false
        i = nextSetBit(i + 1)
      out.append("}").toString

  @javaDefined
  @jvmClass("java/util/UUID")
  final class UUID(mostSigBits: Long, leastSigBits: Long) extends java.lang.Comparable[UUID]:
    def getMostSignificantBits(): Long = mostSigBits
    def getLeastSignificantBits(): Long = leastSigBits
    def version(): Int = ((mostSigBits >> 12) & 0xfL).toInt
    def variant(): Int = ((leastSigBits >>> (64 - (leastSigBits >>> 62)).toInt) & (leastSigBits >> 63)).toInt
    def compareTo(that: UUID): Int =
      val most = java.lang.Long.compare(mostSigBits, that.getMostSignificantBits())
      if most != 0 then most else java.lang.Long.compare(leastSigBits, that.getLeastSignificantBits())
    override def equals(that: Any): Boolean = that match
      case u: UUID => u.getMostSignificantBits() == mostSigBits && u.getLeastSignificantBits() == leastSigBits
      case _ => false
    override def hashCode: Int =
      val hilo = mostSigBits ^ leastSigBits
      (hilo >> 32).toInt ^ hilo.toInt
    override def toString: String =
      UUID.hex(mostSigBits >> 32, 8) + "-" + UUID.hex(mostSigBits >> 16, 4) + "-" + UUID.hex(mostSigBits, 4) + "-" +
        UUID.hex(leastSigBits >> 48, 4) + "-" + UUID.hex(leastSigBits, 12)

  @javaDefined
  @jvmClass("java/util/UUID")
  object UUID:
    private[util] def hex(value: Long, digits: Int): String =
      var text = java.lang.Long.toHexString(value & ((1L << (4 * digits)) - 1))
      while text.length < digits do text = "0" + text
      text

    @jvm("invokestatic java/util/UUID.randomUUID()Ljava/util/UUID;")
    def randomUUID(): UUID = fromString(randomText())

    // Version 4 from the platform's cryptographic source.
    @js("(() => { const c = globalThis.crypto; if (c.randomUUID) return c.randomUUID(); const b = c.getRandomValues(new Uint8Array(16)); b[6] = (b[6] & 15) | 64; b[8] = (b[8] & 63) | 128; const h = Array.from(b, (x) => (x + 256).toString(16).slice(1)).join(''); return h.slice(0, 8) + '-' + h.slice(8, 12) + '-' + h.slice(12, 16) + '-' + h.slice(16, 20) + '-' + h.slice(20); })()")
    private def randomText(): String =
      val most = (scala.math.random() * 4294967296.0).toLong << 32 | (scala.math.random() * 4294967296.0).toLong
      val least = (scala.math.random() * 4294967296.0).toLong << 32 | (scala.math.random() * 4294967296.0).toLong
      new UUID((most & ~0xf000L) | 0x4000L, (least & 0x3fffffffffffffffL) | Long.MinValue).toString

    @jvm("invokestatic java/util/UUID.fromString(Ljava/lang/String;)Ljava/util/UUID;")
    def fromString(name: String): UUID =
      val len = name.length
      if len > 36 then throw new IllegalArgumentException("UUID string too large")
      val dash1 = name.indexOf('-', 0)
      val dash2 = name.indexOf('-', dash1 + 1)
      val dash3 = name.indexOf('-', dash2 + 1)
      val dash4 = name.indexOf('-', dash3 + 1)
      val dash5 = name.indexOf('-', dash4 + 1)
      if dash4 < 0 || dash5 >= 0 then throw new IllegalArgumentException("Invalid UUID string: " + name)
      var most = parseHex(name, 0, dash1) & 0xffffffffL
      most = (most << 16) | (parseHex(name, dash1 + 1, dash2) & 0xffffL)
      most = (most << 16) | (parseHex(name, dash2 + 1, dash3) & 0xffffL)
      var least = parseHex(name, dash3 + 1, dash4) & 0xffffL
      least = (least << 48) | (parseHex(name, dash4 + 1, len) & 0xffffffffffffL)
      new UUID(most, least)

    // `Long.parseLong(s, begin, end, 16)`, accumulating negatively as the JDK does.
    private def parseHex(s: String, begin: Int, end: Int): Long =
      def fail(at: Int): Nothing =
        throw new NumberFormatException("Error at index " + (at - begin) + " in: \"" + s.substring(begin, end) + "\"")
      if begin >= end then throw new NumberFormatException("For input string: \"\" under radix 16")
      var i = begin
      var negative = false
      var limit = -Long.MaxValue
      val first = s.charAt(i)
      if first < '0' then
        if first == '-' then
          negative = true
          limit = Long.MinValue
        else if first != '+' then fail(i)
        i += 1
        if i >= end then fail(i)
      val multmin = limit / 16
      var result = 0L
      while i < end do
        val c = s.charAt(i)
        val digit =
          if c >= '0' && c <= '9' then c - '0'
          else if c >= 'a' && c <= 'f' then c - 'a' + 10
          else if c >= 'A' && c <= 'F' then c - 'A' + 10
          else -1
        if digit < 0 || result < multmin then fail(i)
        result *= 16
        if result < limit + digit then fail(i)
        result -= digit
        i += 1
      if negative then result else -result

  @javaDefined
  @jvmClass("java/util/Base64")
  object Base64:
    private val alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
    private val urlAlphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
    private val basic = new Encoder(false, 0, true)
    private val url = new Encoder(true, 0, true)
    private val mime = new Encoder(false, 76, true)

    @jvm("invokestatic java/util/Base64.getEncoder()Ljava/util/Base64$Encoder;")
    def getEncoder(): Encoder = basic
    @jvm("invokestatic java/util/Base64.getUrlEncoder()Ljava/util/Base64$Encoder;")
    def getUrlEncoder(): Encoder = url
    @jvm("invokestatic java/util/Base64.getMimeEncoder()Ljava/util/Base64$Encoder;")
    def getMimeEncoder(): Encoder = mime
    @jvm("invokestatic java/util/Base64.getDecoder()Ljava/util/Base64$Decoder;")
    def getDecoder(): Decoder = new Decoder(false, false)
    @jvm("invokestatic java/util/Base64.getUrlDecoder()Ljava/util/Base64$Decoder;")
    def getUrlDecoder(): Decoder = new Decoder(true, false)
    @jvm("invokestatic java/util/Base64.getMimeDecoder()Ljava/util/Base64$Decoder;")
    def getMimeDecoder(): Decoder = new Decoder(false, true)

    @javaDefined
    @jvmClass("java/util/Base64$Encoder")
    final class Encoder(isUrl: Boolean, lineMax: Int, padding: Boolean):
      @jvm("$0 invokevirtual java/util/Base64$Encoder.withoutPadding()Ljava/util/Base64$Encoder;")
      def withoutPadding(): Encoder = new Encoder(isUrl, lineMax, false)
      @jvm("$0 rt $1:L rtcall javaBytes(Ljava/lang/Object;)Ljava/lang/Object; checkcast [B invokevirtual java/util/Base64$Encoder.encodeToString([B)Ljava/lang/String;")
      def encodeToString(src: Array[Byte]): String =
        val chars = if isUrl then urlAlphabet else alphabet
        val out = new java.lang.StringBuilder()
        var i = 0
        var line = 0
        while i < src.length do
          val n = src.length - i
          val bits = (src(i) & 255) << 16 | (if n > 1 then (src(i + 1) & 255) << 8 else 0) | (if n > 2 then src(i + 2) & 255 else 0)
          if lineMax > 0 && line == lineMax then
            out.append("\r\n")
            line = 0
          out.append(chars.charAt(bits >> 18 & 63))
          out.append(chars.charAt(bits >> 12 & 63))
          if n > 1 then out.append(chars.charAt(bits >> 6 & 63)) else if padding then out.append('=')
          if n > 2 then out.append(chars.charAt(bits & 63)) else if padding then out.append('=')
          line += 4
          i += 3
        out.toString
      @jvm("$0 $1 invokevirtual java/util/Base64$Encoder.encode([B)[B")
      def encode(src: Array[Byte]): Array[Byte] =
        val text = encodeToString(src)
        Array.tabulate(text.length)(i => text.charAt(i).toByte)

    @javaDefined
    @jvmClass("java/util/Base64$Decoder")
    final class Decoder(isUrl: Boolean, isMime: Boolean):
      @jvm("$0 $1 invokevirtual java/util/Base64$Decoder.decode(Ljava/lang/String;)[B")
      def decode(src: String): Array[Byte] =
        decode(Array.tabulate(src.length)(i => if src.charAt(i) > 255 then '?'.toByte else src.charAt(i).toByte))
      @jvm("$0 $1 invokevirtual java/util/Base64$Decoder.decode([B)[B")
      def decode(src: Array[Byte]): Array[Byte] =
        val chars = if isUrl then urlAlphabet else alphabet
        val out = new Array[Byte](src.length)
        var dp = 0
        def put(b: Int): Unit =
          out(dp) = b.toByte
          dp += 1
        var sp = 0
        var bits = 0
        var shiftto = 18
        var padded = false
        while sp < src.length && !padded do
          val c = src(sp) & 255
          sp += 1
          val b = if c == '='.toInt then -2 else chars.indexOf(c.toChar)
          if b == -2 then
            if shiftto == 6 && (sp == src.length || { sp += 1; src(sp - 1) != '='.toByte }) || shiftto == 18 then
              throw new IllegalArgumentException("Input byte array has wrong 4-byte ending unit")
            padded = true
          else if b < 0 then
            if !isMime then throw new IllegalArgumentException("Illegal base64 character " + java.lang.Integer.toString(src(sp - 1).toInt, 16))
          else
            bits |= b << shiftto
            shiftto -= 6
            if shiftto < 0 then
              put(bits >> 16)
              put(bits >> 8)
              put(bits)
              shiftto = 18
              bits = 0
        if shiftto == 6 then put(bits >> 16)
        else if shiftto == 0 then
          put(bits >> 16)
          put(bits >> 8)
        else if shiftto == 12 then throw new IllegalArgumentException("Last unit does not have enough valid bits")
        while sp < src.length do
          sp += 1
          if !isMime || chars.indexOf((src(sp - 1) & 255).toChar) >= 0 then
            throw new IllegalArgumentException("Input byte array has incorrect ending byte at " + sp)
        java.util.Arrays.copyOf(out, dp)

  @javaDefined
  @jvmClass("java/util/Locale")
  final class Locale(languageText: String, countryText: String):
    def this(language: String) = this(language, "")
    private val lang = languageText.toLowerCase
    private val country = countryText.toUpperCase
    def getLanguage(): String = lang
    def getCountry(): String = country
    def getVariant(): String = ""
    def getUnicodeLocaleType(key: String): String = null
    def toLanguageTag(): String =
      if lang.isEmpty && country.isEmpty then "und"
      else if country.isEmpty then lang
      else (if lang.isEmpty then "und" else lang) + "-" + country
    // Display names in English, of the languages and regions `Locale`'s constants name, whatever
    // the locale asked (a null one refused as the JDK refuses it); another code is its own name, the
    // std carrying no locale data.
    @jvm("invokevirtual java/util/Locale.getDisplayLanguage()Ljava/lang/String;")
    def getDisplayLanguage(): String = getDisplayLanguage(Locale.getDefault())
    @jvm("invokevirtual java/util/Locale.getDisplayLanguage(Ljava/util/Locale;)Ljava/lang/String;")
    def getDisplayLanguage(inLocale: Locale): String =
      if inLocale == null then throw new NullPointerException()
      lang match
        case "en" => "English"
        case "fr" => "French"
        case "de" => "German"
        case "it" => "Italian"
        case "ja" => "Japanese"
        case other => other
    @jvm("invokevirtual java/util/Locale.getDisplayCountry()Ljava/lang/String;")
    def getDisplayCountry(): String = getDisplayCountry(Locale.getDefault())
    @jvm("invokevirtual java/util/Locale.getDisplayCountry(Ljava/util/Locale;)Ljava/lang/String;")
    def getDisplayCountry(inLocale: Locale): String =
      if inLocale == null then throw new NullPointerException()
      country match
        case "US" => "United States"
        case "GB" => "United Kingdom"
        case "CA" => "Canada"
        case "FR" => "France"
        case "DE" => "Germany"
        case "IT" => "Italy"
        case "JP" => "Japan"
        case other => other
    @jvm("invokevirtual java/util/Locale.getDisplayName()Ljava/lang/String;")
    def getDisplayName(): String = getDisplayName(Locale.getDefault())
    @jvm("invokevirtual java/util/Locale.getDisplayName(Ljava/util/Locale;)Ljava/lang/String;")
    def getDisplayName(inLocale: Locale): String =
      // The JDK looks the locale up in a map first, with its message.
      if inLocale == null then throw new NullPointerException("Cannot invoke \"Object.hashCode()\" because \"key\" is null")
      val language = getDisplayLanguage(inLocale)
      val region = getDisplayCountry(inLocale)
      if language.isEmpty then region else if region.isEmpty then language else language + " (" + region + ")"
    override def toString: String = if country.isEmpty then lang else lang + "_" + country
    override def equals(that: Any): Boolean = that match
      case l: Locale => l.getLanguage() == lang && l.getCountry() == country
      case _ => false
    override def hashCode: Int = lang.hashCode * 31 + country.hashCode

  @javaDefined
  @jvmClass("java/util/Locale")
  object Locale:
    private val root = new Locale("", "")
    private val english = new Locale("en", "")
    private val us = new Locale("en", "US")
    private val uk = new Locale("en", "GB")
    private val canada = new Locale("en", "CA")
    private val french = new Locale("fr", "")
    private val france = new Locale("fr", "FR")
    private val german = new Locale("de", "")
    private val germany = new Locale("de", "DE")
    private val italian = new Locale("it", "")
    private val italy = new Locale("it", "IT")
    private val japanese = new Locale("ja", "")
    private val japan = new Locale("ja", "JP")
    @jvm("getstatic java/util/Locale.ROOT:Ljava/util/Locale;")
    def ROOT: Locale = root
    @jvm("getstatic java/util/Locale.ENGLISH:Ljava/util/Locale;")
    def ENGLISH: Locale = english
    @jvm("getstatic java/util/Locale.US:Ljava/util/Locale;")
    def US: Locale = us
    @jvm("getstatic java/util/Locale.UK:Ljava/util/Locale;")
    def UK: Locale = uk
    @jvm("getstatic java/util/Locale.CANADA:Ljava/util/Locale;")
    def CANADA: Locale = canada
    @jvm("getstatic java/util/Locale.FRENCH:Ljava/util/Locale;")
    def FRENCH: Locale = french
    @jvm("getstatic java/util/Locale.FRANCE:Ljava/util/Locale;")
    def FRANCE: Locale = france
    @jvm("getstatic java/util/Locale.GERMAN:Ljava/util/Locale;")
    def GERMAN: Locale = german
    @jvm("getstatic java/util/Locale.GERMANY:Ljava/util/Locale;")
    def GERMANY: Locale = germany
    @jvm("getstatic java/util/Locale.ITALIAN:Ljava/util/Locale;")
    def ITALIAN: Locale = italian
    @jvm("getstatic java/util/Locale.ITALY:Ljava/util/Locale;")
    def ITALY: Locale = italy
    @jvm("getstatic java/util/Locale.JAPANESE:Ljava/util/Locale;")
    def JAPANESE: Locale = japanese
    @jvm("getstatic java/util/Locale.JAPAN:Ljava/util/Locale;")
    def JAPAN: Locale = japan
    @jvm("invokestatic java/util/Locale.of(Ljava/lang/String;Ljava/lang/String;)Ljava/util/Locale;")
    def of(language: String, country: String): Locale = new Locale(language, country)
    @jvm("invokestatic java/util/Locale.of(Ljava/lang/String;)Ljava/util/Locale;")
    def of(language: String): Locale = new Locale(language, "")
    private var default: Locale = us
    @jvm("invokestatic java/util/Locale.getDefault()Ljava/util/Locale;")
    def getDefault(): Locale = default
    @jvm("invokestatic java/util/Locale.setDefault(Ljava/util/Locale;)V")
    def setDefault(locale: Locale): Unit = default = locale
    // The language and region subtags; the rest of a tag is not kept.
    @jvm("invokestatic java/util/Locale.forLanguageTag(Ljava/lang/String;)Ljava/util/Locale;")
    def forLanguageTag(tag: String): Locale =
      val parts = tag.split("[-_]")
      val language = if parts(0) == "und" then "" else parts(0)
      val region = if parts.length > 1 && (parts(1).length == 2 || parts(1).length == 3 && parts(1).forall(_.isDigit)) then parts(1) else ""
      new Locale(language, region)

  // A zone by its id; offsets and rules are `java.time`'s.
  @javaDefined
  @jvmClass("java/util/TimeZone")
  final class TimeZone(id: String):
    def getID(): String = id
    // The zone's offset from UTC in seconds, at an instant and of a local date and time: the machine's
    // own zone by its natives, `UTC`, `GMT` and a fixed offset (`GMT+05:00`) by arithmetic; teq interp
    // has no rules of another region (`java.time`'s `ZoneId.of` refuses it alike).
    private val fixedSeconds: Int = TimeZone.fixedOffset(id)
    private[java] def offsetSeconds(epochSecond: Long): Int =
      if fixedSeconds != TimeZone.NotFixed then fixedSeconds
      else if id == java.util.zip.zoneName() then java.util.zip.localOffset(epochSecond)
      else throw new UnsupportedOperationException("teq interp has the rules of the machine's own zone (" + java.util.zip.zoneName() + ") alone, not " + id)
    private[java] def localSeconds(year: Int, month: Int, day: Int, hour: Int, minute: Int, second: Int): Long =
      if fixedSeconds == TimeZone.NotFixed && id == java.util.zip.zoneName() then java.util.zip.localInstant(year, month, day, hour, minute, second)
      else
        val local = java.time.LocalDateTime.of(year, month, day, hour, minute, second).toEpochSecond(java.time.ZoneOffset.UTC)
        local - offsetSeconds(local)
    // The JDK's: the offset in milliseconds at an instant in milliseconds.
    def getOffset(date: Long): Int = offsetSeconds(Math.floorDiv(date, 1000L)) * 1000
    // `java.time` is the JDK's on the JVM and scala-java-time's from its jar on JavaScript.
    def toZoneId(): java.time.ZoneId = java.time.ZoneId.of(id)
    def getDisplayName(): String = id
    def getDisplayName(daylight: Boolean, style: Int, locale: Locale): String = id
    override def equals(that: Any): Boolean = that match
      case z: TimeZone => z.getID() == id
      case _ => false
    override def hashCode: Int = id.hashCode

  @javaDefined
  @jvmClass("java/util/TimeZone")
  object TimeZone:
    @jvm("getstatic java/util/TimeZone.SHORT:I")
    def SHORT: Int = 0
    @jvm("getstatic java/util/TimeZone.LONG:I")
    def LONG: Int = 1
    private var default: TimeZone = null
    @jvm("invokestatic java/util/TimeZone.getTimeZone(Ljava/lang/String;)Ljava/util/TimeZone;")
    def getTimeZone(id: String): TimeZone = new TimeZone(id)
    // The engine's zone as its offset now (`+02:00`, `Z`): an id `java.time.ZoneId.of`
    // resolves without the tzdb, so `LocalDate.now()` reads the wall clock; a program that
    // registers the tzdb may `setDefault(getTimeZone("Europe/Paris"))` for the region's rules.
    @jvm("invokestatic java/util/TimeZone.getDefault()Ljava/util/TimeZone;")
    def getDefault(): TimeZone = if default == null then new TimeZone(systemOffsetId()) else default
    @jvm("invokestatic java/util/TimeZone.setDefault(Ljava/util/TimeZone;)V")
    def setDefault(zone: TimeZone): Unit = default = zone
    // The machine's zone by its name in teq interp, as the JDK names it (`America/Los_Angeles`).
    @js("(() => { const m = -new Date().getTimezoneOffset(); if (m === 0) return \"Z\"; const a = Math.abs(m); return (m < 0 ? \"-\" : \"+\") + String(Math.floor(a / 60)).padStart(2, \"0\") + \":\" + String(a % 60).padStart(2, \"0\") })()")
    private def systemOffsetId(): String = java.util.zip.zoneName()
    private[util] final val NotFixed = Int.MinValue
    // The offset in seconds of `UTC`, `GMT`, `UT`, `Z` and their offsets (`GMT+05:00`, `UTC-3`,
    // `+01:30`), `NotFixed` for a region's id.
    private[util] def fixedOffset(id: String): Int =
      if id == "UTC" || id == "GMT" || id == "UT" || id == "Z" then 0
      else
        val rest =
          if id.startsWith("UTC") || id.startsWith("GMT") then id.substring(3)
          else if id.startsWith("UT") then id.substring(2)
          else id
        if rest.length < 2 || (rest.charAt(0) != '+' && rest.charAt(0) != '-') then NotFixed
        else
          val sign = if rest.charAt(0) == '-' then -1 else 1
          val digits = rest.substring(1).replace(":", "")
          if digits.isEmpty || digits.length > 6 || !digits.forall(c => c >= '0' && c <= '9') then NotFixed
          else
            val (h, m, sec) =
              if digits.length <= 2 then (digits.toInt, 0, 0)
              else if digits.length <= 4 then (digits.substring(0, digits.length - 2).toInt, digits.substring(digits.length - 2).toInt, 0)
              else (digits.substring(0, digits.length - 4).toInt, digits.substring(digits.length - 4, digits.length - 2).toInt, digits.substring(digits.length - 2).toInt)
            sign * (h * 3600 + m * 60 + sec)

  // A point in time in milliseconds; `java.time` is the JDK's on the JVM and scala-java-time's
  // from its jar on JavaScript.
  @javaDefined
  @jvmClass("java/util/Date")
  final class Date(time: Long) extends java.lang.Comparable[Date]:
    def this() = this(java.lang.System.currentTimeMillis())
    def getTime(): Long = time
    def toInstant(): java.time.Instant = java.time.Instant.ofEpochMilli(time)
    def before(when: Date): Boolean = time < when.getTime()
    def after(when: Date): Boolean = time > when.getTime()
    def compareTo(that: Date): Int = java.lang.Long.compare(time, that.getTime())
    override def equals(that: Any): Boolean = that match
      case d: Date => d.getTime() == time
      case _ => false
    override def hashCode: Int = time.toInt ^ (time >> 32).toInt

  @javaDefined
  @jvmClass("java/util/Date")
  object Date:
    @jvm("invokestatic java/util/Date.from(Ljava/time/Instant;)Ljava/util/Date;")
    def from(instant: java.time.Instant): Date = new Date(instant.toEpochMilli())

package java.util.concurrent:


  @js("null")
  def absent[V]: V

  // A hash map with Java's protocol: `null` for a missing key, `compute` over the old value, over a
  // `java.util.HashMap`'s store; one thread runs, so nothing locks. Its key set, values and entries
  // are the store's views, as the JDK's are: a removal through one, or through its iterator, is the
  // map's, and a view taken earlier sees later changes.
  @jvmClass("java/util/concurrent/ConcurrentHashMap")
  final class ConcurrentHashMap[K, V]() extends java.util.AbstractMap[K, V]:
    def this(initialCapacity: Int) = this()
    def this(m: java.util.Map[? <: K, ? <: V]) =
      this()
      putAll(m)
    private val held = new java.util.HashMap[K, V]()
    def get(key: Any): V = held.get(key)
    def put(key: K, value: V): V = held.put(key, value)
    override def putIfAbsent(key: K, value: V): V =
      if held.containsKey(key) then held.get(key)
      else
        held.put(key, value)
        absent[V]
    def containsKey(key: Any): Boolean = held.containsKey(key)
    override def getOrDefault(key: Any, default: V): V = if held.containsKey(key) then held.get(key) else default
    def remove(key: Any): V = held.remove(key)
    override def compute(key: K, remapping: java.util.function.BiFunction[? >: K, ? >: V, ? <: V]): V =
      val next: V = remapping.asInstanceOf[java.util.function.BiFunction[K, V, V]].apply(key, get(key))
      if js.isNull(next) then held.remove(key) else held.put(key, next)
      next
    override def computeIfAbsent(key: K, mapping: java.util.function.Function[? >: K, ? <: V]): V =
      if held.containsKey(key) then held.get(key)
      else
        val v: V = mapping.asInstanceOf[java.util.function.Function[K, V]].apply(key)
        if !js.isNull(v) then held.put(key, v)
        v
    def size(): Int = held.size()
    override def isEmpty(): Boolean = held.isEmpty()
    def clear(): Unit = held.clear()
    def keySet(): ConcurrentHashMap.KeySetView[K, V] = new ConcurrentHashMap.KeySetView(this, absent[V])
    def keySet(mappedValue: V): ConcurrentHashMap.KeySetView[K, V] =
      java.util.Objects.requireNonNull(mappedValue)
      new ConcurrentHashMap.KeySetView(this, mappedValue)
    def values(): java.util.Collection[V] = held.values()
    def entrySet(): java.util.Set[java.util.Map.Entry[K, V]] = held.entrySet()
    private[concurrent] def keys: java.util.Set[K] = held.keySet()

  @jvmClass("java/util/concurrent/ConcurrentHashMap")
  object ConcurrentHashMap:
    def newKeySet[K](): KeySetView[K, java.lang.Boolean] = new KeySetView(new ConcurrentHashMap[K, java.lang.Boolean](), java.lang.Boolean.valueOf(true))
    def newKeySet[K](initialCapacity: Int): KeySetView[K, java.lang.Boolean] = newKeySet[K]()
    // The JDK's view of a map's keys (with its `CollectionView`'s members): a removal through it is the
    // map's; `add` puts a key with the view's mapped value, which a plain `keySet()` has none of, so
    // that it refuses to add. A `Set` alone among its parents, as the JDK's other parent is no public class.
    @jvmClass("java/util/concurrent/ConcurrentHashMap$KeySetView")
    final class KeySetView[K, V] private[concurrent] (map: ConcurrentHashMap[K, V], value: V) extends java.util.Set[K]:
      def getMappedValue(): V = value
      def getMap(): ConcurrentHashMap[K, V] = map
      def size(): Int = map.size()
      override def isEmpty(): Boolean = map.isEmpty()
      override def clear(): Unit = map.clear()
      override def contains(o: Any): Boolean = map.containsKey(o)
      override def remove(o: Any): Boolean = !js.isNull(map.remove(o))
      def iterator(): java.util.Iterator[K] = map.keys.iterator()
      override def add(e: K): Boolean =
        if js.isNull(value) then throw new UnsupportedOperationException()
        js.isNull(map.putIfAbsent(e, value))
      override def addAll(c: java.util.Collection[? <: K]): Boolean =
        if js.isNull(value) then throw new UnsupportedOperationException()
        var added = false
        val it = c.iterator()
        while it.hasNext do
          if js.isNull(map.putIfAbsent(it.next(), value)) then added = true
        added
      override def hashCode: Int =
        var h = 0
        val it = iterator()
        while it.hasNext do h += it.next().hashCode
        h
      override def equals(o: Any): Boolean = o match
        case c: java.util.Set[?] => (c eq this) || (containsAll(c) && c.containsAll(this))
        case _ => false
      override def toString: String =
        val sb = new java.lang.StringBuilder("[")
        val it = iterator()
        while it.hasNext do
          val e = it.next()
          sb.append(if e.asInstanceOf[AnyRef] eq this then "(this Collection)" else String.valueOf(e))
          if it.hasNext then sb.append(", ")
        sb.append(']').toString

  @javaDefined
  @jvmClass("java/util/concurrent/TimeUnit")
  final class TimeUnit(label: String, position: Int, val scale: Long) extends java.lang.Enum[TimeUnit]:
    override def name(): String = label
    override def ordinal(): Int = position
    override def toString: String = label
    @jvm("$0 $1 invokevirtual java/lang/Enum.compareTo(Ljava/lang/Enum;)I")
    override def compareTo(that: TimeUnit): Int = position - that.ordinal()
    @jvm("$0 invokevirtual java/lang/Enum.hashCode()I")
    override def hashCode: Int = label.hashCode
    @jvm("$0 $1 invokevirtual java/lang/Enum.equals(Ljava/lang/Object;)Z")
    override def equals(that: Any): Boolean = that.asInstanceOf[AnyRef] eq this
    def convert(duration: Long, unit: TimeUnit): Long = TimeUnit.cvt(duration, scale, unit.scale)
    def toNanos(duration: Long): Long = TimeUnit.cvt(duration, 1L, scale)
    def toMicros(duration: Long): Long = TimeUnit.cvt(duration, 1000L, scale)
    def toMillis(duration: Long): Long = TimeUnit.cvt(duration, 1000000L, scale)
    def toSeconds(duration: Long): Long = TimeUnit.cvt(duration, 1000000000L, scale)
    def toMinutes(duration: Long): Long = TimeUnit.cvt(duration, 60000000000L, scale)
    def toHours(duration: Long): Long = TimeUnit.cvt(duration, 3600000000000L, scale)
    def toDays(duration: Long): Long = TimeUnit.cvt(duration, 86400000000000L, scale)

  @javaDefined
  @jvmClass("java/util/concurrent/TimeUnit")
  object TimeUnit:
    // `duration` of `src` nanoseconds each in units of `dst` nanoseconds, saturated as the JDK does.
    private[concurrent] def cvt(duration: Long, dst: Long, src: Long): Long =
      if src == dst then duration
      else if src < dst then duration / (dst / src)
      else
        val ratio = src / dst
        val max = Long.MaxValue / ratio
        if duration > max then Long.MaxValue
        else if duration < -max then Long.MinValue
        else duration * ratio
    private val all = Array(
      new TimeUnit("NANOSECONDS", 0, 1L),
      new TimeUnit("MICROSECONDS", 1, 1000L),
      new TimeUnit("MILLISECONDS", 2, 1000000L),
      new TimeUnit("SECONDS", 3, 1000000000L),
      new TimeUnit("MINUTES", 4, 60000000000L),
      new TimeUnit("HOURS", 5, 3600000000000L),
      new TimeUnit("DAYS", 6, 86400000000000L))
    @jvm("getstatic java/util/concurrent/TimeUnit.NANOSECONDS:Ljava/util/concurrent/TimeUnit;")
    def NANOSECONDS: TimeUnit = all(0)
    @jvm("getstatic java/util/concurrent/TimeUnit.MICROSECONDS:Ljava/util/concurrent/TimeUnit;")
    def MICROSECONDS: TimeUnit = all(1)
    @jvm("getstatic java/util/concurrent/TimeUnit.MILLISECONDS:Ljava/util/concurrent/TimeUnit;")
    def MILLISECONDS: TimeUnit = all(2)
    @jvm("getstatic java/util/concurrent/TimeUnit.SECONDS:Ljava/util/concurrent/TimeUnit;")
    def SECONDS: TimeUnit = all(3)
    @jvm("getstatic java/util/concurrent/TimeUnit.MINUTES:Ljava/util/concurrent/TimeUnit;")
    def MINUTES: TimeUnit = all(4)
    @jvm("getstatic java/util/concurrent/TimeUnit.HOURS:Ljava/util/concurrent/TimeUnit;")
    def HOURS: TimeUnit = all(5)
    @jvm("getstatic java/util/concurrent/TimeUnit.DAYS:Ljava/util/concurrent/TimeUnit;")
    def DAYS: TimeUnit = all(6)
    @jvm("invokestatic java/util/concurrent/TimeUnit.values()[Ljava/util/concurrent/TimeUnit;")
    def values(): Array[TimeUnit] = all.clone()
    @jvm("invokestatic java/util/concurrent/TimeUnit.valueOf(Ljava/lang/String;)Ljava/util/concurrent/TimeUnit;")
    def valueOf(name: String): TimeUnit = all.find(_.name() == name) match
      case Some(u) => u
      case None => throw new IllegalArgumentException("No enum constant java.util.concurrent.TimeUnit." + name)
