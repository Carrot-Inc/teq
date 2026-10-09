package scala

extension (s: String)
  @js("$0.length")
  @jvm("invokevirtual java/lang/String.length()I")
  def size: Int
  @js("$charAt($0, $1)")
  @jvm("invokevirtual java/lang/String.charAt(I)C")
  def apply(i: Int): Char
  @js("$0.substring($1)")
  def drop(n: Int): String = s.substring(scala.runtime.minInt(scala.runtime.maxInt(n, 0), s.length))
  @js("$0.substring(0, $1)")
  def take(n: Int): String = s.substring(0, scala.runtime.minInt(scala.runtime.maxInt(n, 0), s.length))
  @js("$0.substring(0, Math.max(0, $0.length - $1))")
  def dropRight(n: Int): String = s.take(s.length - scala.runtime.maxInt(n, 0))
  @js("$0.substring(Math.max(0, $0.length - $1))")
  def takeRight(n: Int): String = s.drop(s.length - scala.runtime.maxInt(n, 0))
  @js("($0 + $1)")
  @jvm("invokevirtual java/lang/String.concat(Ljava/lang/String;)Ljava/lang/String;")
  def ++(other: String): String
  @js("($0.length === 0 ? $0 : $0[0].toUpperCase() + $0.substring(1))")
  def capitalize: String = if s.isEmpty then s else s.charAt(0).toUpper.toString + s.substring(1)
  @js("($0.length !== 0)")
  def nonEmpty: Boolean = !s.isEmpty
  @js("($0.startsWith($1) ? $0.substring($1.length) : $0)")
  def stripPrefix(prefix: String): String = if s.startsWith(prefix) then s.substring(prefix.length) else s
  @js("($0.endsWith($1) ? $0.substring(0, $0.length - $1.length) : $0)")
  def stripSuffix(suffix: String): String = if s.endsWith(suffix) then s.substring(0, s.length - suffix.length) else s
  @js("$0.slice(Math.max($1, 0), Math.max($2, 0))")
  def slice(from: Int, until: Int): String =
    val lo = scala.runtime.minInt(scala.runtime.maxInt(from, 0), s.length)
    val hi = scala.runtime.minInt(scala.runtime.maxInt(until, lo), s.length)
    s.substring(lo, hi)
  @js("$format($0, $1)")
  @jvm("$0 $1 invokestatic java/lang/String.format(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;")
  def formatImpl(args: Array[Any]): String
  @js("$parseInt($0)")
  @jvm("invokestatic java/lang/Integer.parseInt(Ljava/lang/String;)I")
  def toInt: Int
  @js("$parseLong($0)")
  @jvm("invokestatic java/lang/Long.parseLong(Ljava/lang/String;)J")
  def toLong: Long
  @js("$parseDouble($0)")
  @jvm("invokestatic java/lang/Double.parseDouble(Ljava/lang/String;)D")
  def toDouble: Double
  // scala-library's: `java.lang.Float.parseFloat`, the JDK's `FloatingDecimal`.
  @jvm("invokestatic java/lang/Float.parseFloat(Ljava/lang/String;)F")
  def toFloat: Float = java.lang.Float.parseFloat(s)
  @js("$parseInt($0) << 24 >> 24")
  @jvm("invokestatic java/lang/Byte.parseByte(Ljava/lang/String;)B")
  def toByte: Byte
  @js("$parseInt($0) << 16 >> 16")
  @jvm("invokestatic java/lang/Short.parseShort(Ljava/lang/String;)S")
  def toShort: Short
  @js("$0.repeat(Math.max(0, $1))")
  def *(n: Int): String = s.repeat(scala.runtime.maxInt(n, 0))
  @js("[...$0].reverse().join(\"\")")
  @jvm("new java/lang/StringBuilder dup $0 invokespecial java/lang/StringBuilder.<init>(Ljava/lang/String;)V invokevirtual java/lang/StringBuilder.reverse()Ljava/lang/StringBuilder; invokevirtual java/lang/StringBuilder.toString()Ljava/lang/String;")
  def reverse: String
  @js("$0.split(/\\r?\\n/)")
  def linesArray: Array[String] = scala.runtime.splitString(s, "\\r?\\n", -1)
  @js("$compareStrings($0, $1)")
  @jvm("invokevirtual java/lang/String.compareTo(Ljava/lang/String;)I")
  def compare(that: String): Int
  @js("$stripMargin($0, $1)")
  def stripMargin(marginChar: Char): String = scala.runtime.stripMarginOf(s, marginChar)
  @js("$stripMargin($0, \"|\")")
  def stripMargin: String = scala.runtime.stripMarginOf(s, '|')
  def head: Char = if s.isEmpty then noSuchElement("head of empty String") else s(0)
  def last: Char = if s.isEmpty then noSuchElement("last of empty String") else s(s.length - 1)
  def tail: String = if s.isEmpty then unsupportedOperation("tail of empty String") else s.drop(1)
  def init: String = if s.isEmpty then unsupportedOperation("init of empty String") else s.dropRight(1)
  def headOption: Option[Char] = if s.isEmpty then None else Some(s(0))
  def lastOption: Option[Char] = if s.isEmpty then None else Some(s(s.length - 1))
  def lines: List[String] = s.terminatedLines.toList
  def linesIterator: Iterator[String] = arrayIterator(s.terminatedLines)
  // A line terminator ends a line rather than starting an empty one.
  private def terminatedLines: Array[String] =
    val parts = s.linesArray
    if parts(parts.length - 1).isEmpty then parts.take(parts.length - 1) else parts
  def r: scala.util.matching.Regex = new scala.util.matching.Regex(s)
  def format(args: Any*): String = s.formatImpl(taggedArray(iterableToArray(args)))
  def toArray: Array[Char] = s.toCharArray
  def toVector: Vector[Char] = Vector.wrap(buffered(s.toCharArray))
  def toSeq: Seq[Char] = Vector.wrap(buffered(s.toCharArray))
  def toIndexedSeq: IndexedSeq[Char] = Vector.wrap(buffered(s.toCharArray))
  def toSet: Set[Char] = Set.from(Vector.wrap(buffered(s.toCharArray)))
  def iterator: Iterator[Char] = arrayIterator(s.toCharArray)
  def find(p: Char => Boolean): Option[Char] = s.toCharArray.find(p)
  def indexWhere(p: Char => Boolean, from: Int = 0): Int =
    var i = if from < 0 then 0 else from
    val n = s.length
    while i < n && !p(s.charAt(i)) do i += 1
    if i < n then i else -1
  def filterNot(p: Char => Boolean): String = s.filter(c => !p(c))
  def takeWhile(p: Char => Boolean): String =
    val i = s.indexWhere(c => !p(c))
    if i < 0 then s else s.take(i)
  def dropWhile(p: Char => Boolean): String =
    val i = s.indexWhere(c => !p(c))
    if i < 0 then "" else s.drop(i)
  def span(p: Char => Boolean): (String, String) = (s.takeWhile(p), s.dropWhile(p))
  def splitAt(n: Int): (String, String) = (s.take(n), s.drop(n))
  def partition(p: Char => Boolean): (String, String) = (s.filter(p), s.filterNot(p))
  def foldLeft[B](z: B)(op: (B, Char) => B): B = s.toCharArray.foldLeft(z)(op)
  def flatMap(f: Char => String): String = s.toCharArray.map(f).mkString
  def grouped(size: Int): Iterator[String] = Vector.wrap(buffered(s.toCharArray)).grouped(size).map(g => g.mkString)
  def sliding(size: Int, step: Int = 1): Iterator[String] = Vector.wrap(buffered(s.toCharArray)).sliding(size, step).map(g => g.mkString)
  def distinct: String = s.toCharArray.distinct.mkString
  def sorted: String = s.toCharArray.sorted.mkString
  // scala-library sizes a builder by the result's length, which throws where more are replaced
  // than there are to replace.
  def patch(from: Int, other: String, replaced: Int): String =
    val size = s.length + other.length - replaced
    if size < 0 then throw new NegativeArraySizeException(size.toString)
    val start = if from > 0 then Math.min(from, s.length) else 0
    val rest = s.length - start - replaced
    s.substring(0, start) + other + (if rest > 0 then s.substring(s.length - rest) else "")
  def patch(from: Int, other: IterableOnce[Char], replaced: Int): String = s.patch(from, other.iterator.mkString, replaced)
  def patch[B >: Char](from: Int, other: IterableOnce[B], replaced: Int): IndexedSeq[B] =
    Vector.wrap(buffered(s.toCharArray)).patch(from, other, replaced)
  def updated(index: Int, elem: Char): String =
    if index < 0 || index >= s.length then
      throw new StringIndexOutOfBoundsException(s"Index $index out of bounds for length ${s.length}")
    s.substring(0, index) + elem + s.substring(index + 1)
  def padTo(len: Int, elem: Char): String = if s.length >= len then s else s + elem.toString * (len - s.length)
  def toList: List[Char] =
    var acc: List[Char] = Nil
    var i = s.length - 1
    while i >= 0 do
      acc = s(i) :: acc
      i -= 1
    acc
  def foreach[U](f: Char => U): Unit =
    var i = 0
    while i < s.length do
      f(s(i))
      i += 1
  def exists(p: Char => Boolean): Boolean =
    var i = 0
    var found = false
    while !found && i < s.length do
      found = p(s(i))
      i += 1
    found
  def forall(p: Char => Boolean): Boolean = !s.exists(c => !p(c))
  def count(p: Char => Boolean): Int =
    var n = 0
    var i = 0
    while i < s.length do
      if p(s(i)) then n += 1
      i += 1
    n
  def filter(p: Char => Boolean): String =
    var out = ""
    var i = 0
    while i < s.length do
      if p(s(i)) then out = out + s(i).toString
      i += 1
    out
  def map(f: Char => Char): String =
    var out = ""
    var i = 0
    while i < s.length do
      out = out + f(s(i)).toString
      i += 1
    out
  def map[B](f: Char => B): IndexedSeq[B] = ArraySeq.unsafeWrapArray(untaggedArray(buffered(s.toCharArray).map(f)))
  def zipWithIndex: List[(Char, Int)] = s.toList.zipWithIndex
  def zip[B](that: IterableOnce[B]): IndexedSeq[(Char, B)] = Vector.wrap(buffered(s.toCharArray)).zip(that)
  def intersect(that: String): String = Vector.wrap(buffered(s.toCharArray)).intersect(Vector.wrap(buffered(that.toCharArray))).mkString
  def diff(that: String): String = Vector.wrap(buffered(s.toCharArray)).diff(Vector.wrap(buffered(that.toCharArray))).mkString
  def indices: Range = Range(0, s.length, 1)
  def toIntOption: Option[Int] = Option(intOrNull(s))
  def toLongOption: Option[Long] = Option(longOrNull(s))
  def toDoubleOption: Option[Double] = Option(doubleOrNull(s))
  def toBooleanOption: Option[Boolean] =
    if s.equalsIgnoreCase("true") then Some(true) else if s.equalsIgnoreCase("false") then Some(false) else None
  def toBoolean: Boolean = s.toBooleanOption match
    case Some(b) => b
    case None => illegalArgument("For input string: \"" + s + "\"")

extension (s: String)
  def mkString(sep: String): String = s.toCharArray.mkString(sep)
  def mkString: String = s
  // The lines with the line break that ends each (`\n`, `\r` or `\r\n`).
  def linesWithSeparators: Iterator[String] =
    var index = 0
    val len = s.length
    def next(): String =
      if index >= len then Iterator.exhausted
      val start = index
      while index < len && s.charAt(index) != '\n' && s.charAt(index) != '\r' do index += 1
      if index < len then
        val c = s.charAt(index)
        index += 1
        if c == '\r' && index < len && s.charAt(index) == '\n' then index += 1
      s.substring(start, index)
    new FnIterator(() => index < len, () => next())

/** scala-library's `Predef.wrapString`: a string where a collection of characters is expected. */
@predef
given wrapString: Conversion[String, IndexedSeq[Char]] = s => Vector.wrap(buffered(s.toCharArray))

/** The chars of `s` in the other order. */
@js("$0.split(\"\").reverse().join(\"\")")
def reverseChars(s: String): String =
  val out = new java.lang.StringBuilder()
  var i = s.length
  while i > 0 do
    i -= 1
    out.append(s.charAt(i))
  out.toString

@js("$intOrNull($0)")
@jvm("rt $0 rtcall intOrNull(Ljava/lang/String;)Ljava/lang/Object;")
def intOrNull(s: String): Int

@js("$longOrNull($0)")
@jvm("rt $0 rtcall longOrNull(Ljava/lang/String;)Ljava/lang/Object;")
def longOrNull(s: String): Long

@js("$doubleOrNull($0)")
@jvm("rt $0 rtcall doubleOrNull(Ljava/lang/String;)Ljava/lang/Object;")
def doubleOrNull(s: String): Double

extension (c: Char)
  @js("/\\p{Nd}/u.test($0)")
  @jvm("invokestatic java/lang/Character.isDigit(C)Z")
  def isDigit: Boolean
  @js("/\\p{L}/u.test($0)")
  @jvm("invokestatic java/lang/Character.isLetter(C)Z")
  def isLetter: Boolean
  @js("/[\\p{L}\\p{Nd}]/u.test($0)")
  @jvm("invokestatic java/lang/Character.isLetterOrDigit(C)Z")
  def isLetterOrDigit: Boolean
  @js("$isWhitespace($0)")
  @jvm("invokestatic java/lang/Character.isWhitespace(C)Z")
  def isWhitespace: Boolean
  @js("/[\\x00-\\x1f\\x7f-\\x9f]/.test($0)")
  @jvm("invokestatic java/lang/Character.isISOControl(C)Z")
  def isControl: Boolean
  @js("/[ \\xa0\\u1680\\u2000-\\u200a\\u2028\\u2029\\u202f\\u205f\\u3000]/.test($0)")
  @jvm("invokestatic java/lang/Character.isSpaceChar(C)Z")
  def isSpaceChar: Boolean
  @js("($0 !== $0.toLowerCase())")
  @jvm("invokestatic java/lang/Character.isUpperCase(C)Z")
  def isUpper: Boolean
  @js("($0 !== $0.toUpperCase())")
  @jvm("invokestatic java/lang/Character.isLowerCase(C)Z")
  def isLower: Boolean
  @js("$0.toUpperCase()")
  @jvm("invokestatic java/lang/Character.toUpperCase(C)C")
  def toUpper: Char
  @js("$0.toLowerCase()")
  @jvm("invokestatic java/lang/Character.toLowerCase(C)C")
  def toLower: Char
  @js("$digit($0, 36)")
  @jvm("$0 ipush 36 invokestatic java/lang/Character.digit(CI)I")
  def asDigit: Int

// On the JVM the builder is the JDK's own (`@jvmClass`): every member has a `@jvm` template, and
// its Scala body is what JavaScript and the interpreter run.
@jvmClass("java/lang/StringBuilder")
final class StringBuilder(init: String = "") extends java.lang.CharSequence, java.lang.Appendable:
  def this(capacity: Int) = this("")
  private var text = init
  @jvm("$0 $1 builder_append")
  def append(x: Any): StringBuilder =
    text = text + x.toString
    this
  @jvm("invokevirtual java/lang/StringBuilder.append(Ljava/lang/String;)Ljava/lang/StringBuilder;")
  def ++=(s: String): StringBuilder = append(s)
  @jvm("invokevirtual java/lang/StringBuilder.append(C)Ljava/lang/StringBuilder;")
  def +=(c: Char): StringBuilder = append(c)
  @jvm("invokevirtual java/lang/StringBuilder.append(C)Ljava/lang/StringBuilder;")
  def addOne(c: Char): StringBuilder = append(c)
  @jvm("invokevirtual java/lang/StringBuilder.append(Ljava/lang/String;)Ljava/lang/StringBuilder;")
  def addAll(s: String): StringBuilder = append(s)
  @jvm("invokevirtual java/lang/StringBuilder.append(Ljava/lang/String;)Ljava/lang/StringBuilder;")
  def appendAll(s: String): StringBuilder = append(s)
  @jvm("$0 $1:I $2 builder_insert")
  def insert(index: Int, x: Any): StringBuilder =
    text = text.substring(0, index) + x.toString + text.substring(index)
    this
  @jvm("invokevirtual java/lang/StringBuilder.deleteCharAt(I)Ljava/lang/StringBuilder;")
  def deleteCharAt(index: Int): StringBuilder =
    text = text.substring(0, index) + text.substring(index + 1)
    this
  @jvm("invokevirtual java/lang/StringBuilder.delete(II)Ljava/lang/StringBuilder;")
  def delete(start: Int, end: Int): StringBuilder =
    text = text.substring(0, start) + text.substring(Math.min(end, text.length))
    this
  @jvm("invokevirtual java/lang/StringBuilder.setLength(I)V")
  def setLength(n: Int): Unit = text = if n <= text.length then text.substring(0, n) else text + "\u0000" * (n - text.length)
  // A new builder of the chars in the other order, a pair of surrogates taken apart as
  // scala-library's builder takes it; on the JVM the JDK's `reverse`, which keeps the pair, turns
  // a copy around.
  @jvm("new java/lang/StringBuilder dup $0:Ljava/lang/CharSequence; invokespecial java/lang/StringBuilder.<init>(Ljava/lang/CharSequence;)V invokevirtual java/lang/StringBuilder.reverse()Ljava/lang/StringBuilder;")
  def reverse: StringBuilder = new StringBuilder(reverseChars(text))
  @jvm("invokevirtual java/lang/StringBuilder.length()I")
  @javaDefined def length: Int = text.length
  @jvm("invokevirtual java/lang/StringBuilder.length()I")
  def size: Int = text.length
  @jvm("invokevirtual java/lang/StringBuilder.isEmpty()Z")
  override def isEmpty: Boolean = text.isEmpty
  @jvm("$0 invokevirtual java/lang/StringBuilder.isEmpty()Z iconst_1 ixor")
  def nonEmpty: Boolean = text.nonEmpty
  @jvm("invokevirtual java/lang/StringBuilder.charAt(I)C")
  def charAt(i: Int): Char = text.charAt(i)
  @jvm("invokevirtual java/lang/StringBuilder.charAt(I)C")
  def apply(i: Int): Char = text.charAt(i)
  @jvm("invokevirtual java/lang/StringBuilder.indexOf(Ljava/lang/String;)I")
  def indexOf(part: String): Int = text.indexOf(part)
  @jvm("invokevirtual java/lang/StringBuilder.lastIndexOf(Ljava/lang/String;)I")
  def lastIndexOf(part: String): Int = text.lastIndexOf(part)
  @jvm("invokevirtual java/lang/StringBuilder.substring(II)Ljava/lang/String;")
  def substring(start: Int, end: Int): String = text.substring(start, end)
  @jvm("invokevirtual java/lang/StringBuilder.subSequence(II)Ljava/lang/CharSequence;")
  def subSequence(start: Int, end: Int): java.lang.CharSequence = text.substring(start, end)
  @jvm("rt $0 $1:I rtcall builderTake(Ljava/lang/StringBuilder;I)Ljava/lang/StringBuilder;")
  def take(n: Int): StringBuilder = new StringBuilder(text.take(n))
  @jvm("rt $0 $1:I rtcall builderDrop(Ljava/lang/StringBuilder;I)Ljava/lang/StringBuilder;")
  def drop(n: Int): StringBuilder = new StringBuilder(text.drop(n))
  @jvm("rt $0 $1:I rtcall builderDropRight(Ljava/lang/StringBuilder;I)Ljava/lang/StringBuilder;")
  def dropRight(n: Int): StringBuilder = new StringBuilder(text.dropRight(n))
  @jvm("rt $0 $1:I rtcall builderTakeRight(Ljava/lang/StringBuilder;I)Ljava/lang/StringBuilder;")
  def takeRight(n: Int): StringBuilder = new StringBuilder(text.takeRight(n))
  @jvm("$0 iconst_0 invokevirtual java/lang/StringBuilder.charAt(I)C")
  def head: Char = text.head
  @jvm("$0 dup invokevirtual java/lang/StringBuilder.length()I iconst_1 isub invokevirtual java/lang/StringBuilder.charAt(I)C")
  def last: Char = text.last
  @jvm("$0 invokevirtual java/lang/StringBuilder.toString()Ljava/lang/String; $1 invokevirtual java/lang/String.startsWith(Ljava/lang/String;)Z")
  def startsWith(prefix: String): Boolean = text.startsWith(prefix)
  @jvm("$0 invokevirtual java/lang/StringBuilder.toString()Ljava/lang/String; $1 invokevirtual java/lang/String.endsWith(Ljava/lang/String;)Z")
  def endsWith(suffix: String): Boolean = text.endsWith(suffix)
  // Seq.contains of Scala: an element, so only a Char can be found.
  @jvm("rt $0 $1:L rtcall builderContains(Ljava/lang/StringBuilder;Ljava/lang/Object;)Z")
  def contains(elem: Any): Boolean = elem match
    case c: Char if c.toString.length == 1 => text.contains(c.toString)
    case _ => false
  @jvm("invokevirtual java/lang/StringBuilder.setCharAt(IC)V")
  def update(index: Int, c: Char): Unit = text = text.substring(0, index) + c.toString + text.substring(index + 1)
  @jvm("invokevirtual java/lang/StringBuilder.setCharAt(IC)V")
  def setCharAt(index: Int, c: Char): Unit = update(index, c)
  @jvm("invokevirtual java/lang/StringBuilder.replace(IILjava/lang/String;)Ljava/lang/StringBuilder;")
  def replace(start: Int, end: Int, str: String): StringBuilder =
    text = text.substring(0, start) + str + text.substring(if end > text.length then text.length else end)
    this
  @jvm("rt $0 $1 rtcall builderForeach(Ljava/lang/StringBuilder;Lscala/Function1;)V")
  def foreach[U](f: Char => U): Unit = text.foreach(f)
  @jvm("rt $0 rtcall builderToList(Ljava/lang/StringBuilder;)Lscala/List;")
  def toList: List[Char] = text.toList
  @jvm("$0 iconst_0 invokevirtual java/lang/StringBuilder.setLength(I)V")
  def clear(): Unit = text = ""
  @jvm("invokevirtual java/lang/StringBuilder.toString()Ljava/lang/String;")
  def result(): String = text
  @jvm("invokevirtual java/lang/StringBuilder.toString()Ljava/lang/String;")
  def mkString: String = text
  @jvm("invokevirtual java/lang/StringBuilder.toString()Ljava/lang/String;")
  override def toString: String = text

final class StringContext(val parts: String*):
  private def interleave(args: Seq[Any], escaped: Boolean): String =
    if parts.length != args.length + 1 then
      sys.error("wrong number of arguments (" + args.length.toString + ") for interpolated string with " + parts.length.toString + " parts")
    var out = ""
    var i = 0
    while i < parts.length do
      out = out + (if escaped then StringContext.processEscapes(parts(i)) else parts(i))
      if i < args.length then out = out + args(i).toString
      i += 1
    out
  def s(args: Any*): String = interleave(args, true)
  def raw(args: Any*): String = interleave(args, false)
  /** `case s"a$x"`: the extractor of the `s` interpolator, scalac's `StringContext.s`. */
  def s: StringContext.Extractor = new StringContext.Extractor(parts)
  def f[A >: Any](args: A*): String = StringContext.formatInterpolated(taggedArray(iterableToArray(parts)), taggedArray[Any](iterableToArray(args)))

object StringContext:
  final class Extractor(parts: Seq[String]):
    def unapplySeq(s: String): Option[Seq[String]] = glob(parts, s)
  /** The pattern `s"a$x-$y"` against `input`, as scala-library's `StringContext.glob`: the
    * literal chunks in order, each hole taking what lies between as little as possible.
    */
  def glob(patternChunks: Seq[String], input: String): Option[Seq[String]] =
    val chunks = patternChunks.toIndexedSeq
    val numWildcards = chunks.length - 1
    val matchStarts = new Array[Int](numWildcards)
    val matchEnds = new Array[Int](numWildcards)
    var i = 0
    while i < numWildcards do
      matchStarts(i) = -1
      matchEnds(i) = -1
      i += 1
    val nameLength = input.length
    var patternLength = numWildcards
    for chunk <- chunks do patternLength += chunk.length
    val pattern = new Array[Int](patternLength)
    val matchIndices = new Array[Int](patternLength + 1)
    i = 0
    while i <= patternLength do
      matchIndices(i) = -1
      i += 1
    i = 0
    var chunkIndex = 0
    while chunkIndex < chunks.length do
      if chunkIndex > 0 then
        pattern(i) = -1
        i += 1
      val chunk = chunks(chunkIndex)
      var j = 0
      while j < chunk.length do
        pattern(i) = chunk.charAt(j).toInt
        i += 1
        j += 1
      if chunkIndex < numWildcards then matchIndices(i) = chunkIndex
      chunkIndex += 1
    var patternIndex = 0
    var inputIndex = 0
    var nextPatternIndex = 0
    var nextInputIndex = 0
    var failed = false
    while !failed && (patternIndex < patternLength || inputIndex < nameLength) do
      val n = matchIndices(patternIndex)
      if n != -1 then
        matchStarts(n) = if matchStarts(n) == -1 then inputIndex else math.min(matchStarts(n), inputIndex)
        matchEnds(n) = if matchEnds(n) == -1 then inputIndex else math.max(matchEnds(n), inputIndex)
      var continued = false
      if patternIndex < patternLength then
        val c = pattern(patternIndex)
        if c == -1 then
          nextPatternIndex = patternIndex
          nextInputIndex = inputIndex + 1
          patternIndex += 1
          continued = true
        else if inputIndex < nameLength && input.charAt(inputIndex).toInt == c then
          patternIndex += 1
          inputIndex += 1
          continued = true
      if !continued then
        if 0 < nextInputIndex && nextInputIndex <= nameLength then
          patternIndex = nextPatternIndex
          inputIndex = nextInputIndex
        else failed = true
    if failed then None
    else
      val out = new Array[String](numWildcards)
      i = 0
      while i < numWildcards do
        out(i) = input.substring(matchStarts(i), matchEnds(i))
        i += 1
      Some(out.toIndexedSeq)
  @js("$processEscapes($1)")
  @jvm("$1 invokevirtual java/lang/String.translateEscapes()Ljava/lang/String;")
  def processEscapes(str: String): String
  @js("$formatInterpolated($1, $2)")
  def formatInterpolated(parts: Array[String], args: Array[Any]): String = scala.runtime.formatInterpolatedParts(parts, args)

/** scala-library's `Symbol`: a name, interned, so that two symbols of one name are one. */
final class Symbol private (val name: String) extends Serializable:
  override def toString: String = "Symbol(" + name + ")"
  override def hashCode: Int = name.hashCode

object Symbol:
  private val interned = scala.collection.mutable.HashMap.empty[String, Symbol]
  def apply(name: String): Symbol = interned.getOrElseUpdate(name, new Symbol(name))
