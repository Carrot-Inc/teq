package scala.runtime

// What generated JVM code calls: cooperative equality, the hashes of scala.runtime.Statics and
// MurmurHash3, sequence patterns, enum lookups and the class behind a partial function literal.
// The JS runtime (runtime/rt.js) holds the counterparts for the other target.

@jvm("$0:L $1:L invokevirtual java/lang/Object.equals(Ljava/lang/Object;)Z")
def objectEquals(a: Any, b: Any): Boolean
@jvm("$0:L invokevirtual java/lang/Object.hashCode()I")
def objectHashCode(a: Any): Int
@jvm("$0:L invokestatic java/util/Objects.isNull(Ljava/lang/Object;)Z")
def isNull(a: Any): Boolean
@jvm("$0:L instanceof java/lang/Number")
def isNumber(a: Any): Boolean
@jvm("$0:L instanceof java/lang/Character")
def isCharacter(a: Any): Boolean
@jvm("$0:L instanceof java/lang/Integer")
def isBoxedInt(a: Any): Boolean
@jvm("$0:L instanceof java/lang/Long")
def isBoxedLong(a: Any): Boolean
@jvm("$0:L instanceof java/lang/Double")
def isBoxedDouble(a: Any): Boolean
@jvm("$0:L instanceof java/lang/Float")
def isBoxedFloat(a: Any): Boolean
@jvm("$0:L instanceof java/lang/Byte")
def isBoxedByte(a: Any): Boolean
@jvm("$0:L instanceof java/lang/Short")
def isBoxedShort(a: Any): Boolean
@jvm("$0:L instanceof java/lang/Boolean")
def isBoxedBoolean(a: Any): Boolean
@jvm("$0:L checkcast java/lang/Number invokevirtual java/lang/Number.intValue()I")
def intValue(a: Any): Int
@jvm("$0:L checkcast java/lang/Number invokevirtual java/lang/Number.longValue()J")
def longValue(a: Any): Long
@jvm("$0:L checkcast java/lang/Number invokevirtual java/lang/Number.doubleValue()D")
def doubleValue(a: Any): Double
@jvm("$0:L checkcast java/lang/Character invokevirtual java/lang/Character.charValue()C i2l")
def charAsLong(a: Any): Long
@jvm("$0:D d2f f2d")
def throughFloat(d: Double): Double
@jvm("$0:D d2f invokestatic java/lang/Float.hashCode(F)I")
def floatHash(d: Double): Int
@jvm("$0:D invokestatic java/lang/Double.hashCode(D)I")
def doubleBitsHash(d: Double): Int
@jvm("$0:J invokestatic java/lang/Long.hashCode(J)I")
def longBitsHash(l: Long): Int
@jvm("$0:I $1:I invokestatic java/lang/Integer.rotateLeft(II)I")
def rotl(i: Int, distance: Int): Int

// scala.runtime.BoxesRunTime.equals: boxed numbers compare by value across their classes.
def equal(a: Any, b: Any): Boolean =
  if a eq b then true
  else if isNull(a) then false
  else if isNumber(a) then
    if isNumber(b) then numbersEqual(a, b)
    else if isCharacter(b) then !isBoxedDouble(a) && longValue(a) == charAsLong(b)
    else objectEquals(a, b)
  else if isCharacter(a) then
    if isNumber(b) then !isBoxedDouble(b) && longValue(b) == charAsLong(a)
    else objectEquals(a, b)
  else objectEquals(a, b)

def isBoxedIntegral(a: Any): Boolean = isBoxedInt(a) || isBoxedLong(a) || isBoxedByte(a) || isBoxedShort(a)
def isBoxedFractional(a: Any): Boolean = isBoxedDouble(a) || isBoxedFloat(a)

def numbersEqual(a: Any, b: Any): Boolean =
  if isBoxedFractional(a) || isBoxedFractional(b) then
    if isBoxedFractional(a) || isBoxedIntegral(a) then
      if isBoxedFractional(b) || isBoxedIntegral(b) then doubleValue(a) == doubleValue(b)
      else objectEquals(b, a)
    else objectEquals(a, b)
  else if isBoxedIntegral(a) && isBoxedIntegral(b) then longValue(a) == longValue(b)
  // A `BigInt` or `BigDecimal` compares itself with a boxed number.
  else if isBoxedIntegral(a) then objectEquals(b, a)
  else objectEquals(a, b)

// scala.runtime.Statics.anyHash: a whole Double hashes as the Int or Long it equals.
def anyHash(x: Any): Int =
  if isNull(x) then 0
  else if isBoxedLong(x) then longHash(longValue(x))
  else if isBoxedDouble(x) then doubleHash(doubleValue(x))
  else if isBoxedFloat(x) then floatHash(doubleValue(x))
  else objectHashCode(x)

def longHash(lv: Long): Int =
  val iv = lv.toInt
  if iv.toLong == lv then iv else longBitsHash(lv)

def doubleHash(dv: Double): Int =
  val iv = dv.toInt
  if iv.toDouble == dv then iv
  else
    val lv = dv.toLong
    if lv.toDouble == dv then longBitsHash(lv)
    else if throughFloat(dv) == dv then floatHash(dv)
    else doubleBitsHash(dv)

def mixLast(hash: Int, data: Int): Int =
  var k = data
  k = k * -862048943
  k = rotl(k, 15)
  k = k * 461845907
  hash ^ k

def mix(hash: Int, data: Int): Int =
  var h = mixLast(hash, data)
  h = rotl(h, 13)
  h * 5 + -430675100

def avalanche(h0: Int): Int =
  var h = h0
  h = h ^ (h >>> 16)
  h = h * -2048144789
  h = h ^ (h >>> 13)
  h = h * -1028477387
  h ^ (h >>> 16)

def finalizeHash(hash: Int, length: Int): Int = avalanche(hash ^ length)

@jvm("$0:L checkcast scala/Product invokeinterface scala/Product.productPrefix()Ljava/lang/String;")
def productPrefixOf(x: Any): String

@jvm("$0:L invokevirtual java/lang/Object.getClass()Ljava/lang/Class; invokevirtual java/lang/Class.getName()Ljava/lang/String;")
def classNameOf(x: Any): String

def matchErrorMessage(obj: Any): String = stringOf(obj) + " (of class " + classNameOf(obj) + ")"

def enumValueOf(values: Array[Any], name: String, enumName: String): Any =
  var i = 0
  var found = -1
  while found < 0 && i < values.length do
    if productPrefixOf(values(i)) == name then found = i
    i += 1
  if found < 0 then throw new IllegalArgumentException("enum " + enumName + " has no case with name: " + name)
  values(found)

def enumFromOrdinal(values: Array[Any], ordinal: Int, enumName: String): Any =
  if ordinal < 0 || ordinal >= values.length then
    throw new NoSuchElementException("enum " + enumName + " has no case with ordinal: " + ordinal.toString)
  values(ordinal)

// The first `n` elements of a sequence behind a `true`, then the rest of it when `rest` is set;
// empty when the sequence is shorter, or longer without `rest`.
def seqPattern(x: Any, n: Int, rest: Boolean): Array[Any] =
  val out = new Array[Any](if rest then n + 2 else n + 1)
  out(0) = true
  x match
    case arr: Array[?] =>
      val len = arr.length
      if (if rest then len < n else len != n) then new Array[Any](0)
      else
        var i = 0
        while i < n do
          out(i + 1) = arr(i)
          i += 1
        if rest then out(n + 1) = arrayRest(arr, n, len)
        out
    case seq: Seq[?] =>
      val it = seq.iterator
      var ok = true
      var i = 0
      while ok && i < n do
        if it.hasNext then out(i + 1) = it.next() else ok = false
        i += 1
      if !ok then new Array[Any](0)
      else if rest then
        out(n + 1) = seq.drop(n)
        out
      else if it.hasNext then new Array[Any](0)
      else out
    case _ => new Array[Any](0)

final class PartialFunctionImpl[A, B](total: (A, A => B) => B, defined: A => Boolean) extends PartialFunction[A, B]:
  def apply(x: A): B = total(x, v => matchFailed(v))
  def isDefinedAt(x: A): Boolean = defined(x)
  override def applyOrElse[A1 <: A, B1 >: B](x: A1, default: A1 => B1): B1 =
    unsafeCast(total(x, unsafeCast(default)))

@jvm("new scala/MatchError dup $0:L invokespecial scala/MatchError.<init>(Ljava/lang/Object;)V athrow")
def matchFailed(v: Any): Nothing

// The default that tells "no case matched" from a result.
object Missed
val missFunction: Any => Any = _ => Missed

// ---- numbers ----

@jvm("$0:I $1:I invokestatic java/lang/Math.min(II)I")
def minInt(a: Int, b: Int): Int
@jvm("$0:I $1:I invokestatic java/lang/Math.max(II)I")
def maxInt(a: Int, b: Int): Int
@jvm("$0:J $1:J invokestatic java/lang/Math.min(JJ)J")
def minLong(a: Long, b: Long): Long
@jvm("$0:J $1:J invokestatic java/lang/Math.max(JJ)J")
def maxLong(a: Long, b: Long): Long
@jvm("$0:D $1:D invokestatic java/lang/Math.min(DD)D")
def minDouble(a: Double, b: Double): Double
@jvm("$0:D $1:D invokestatic java/lang/Math.max(DD)D")
def maxDouble(a: Double, b: Double): Double
@jvm("$0:I invokestatic java/lang/Math.abs(I)I")
def absInt(a: Int): Int
@jvm("$0:J invokestatic java/lang/Math.abs(J)J")
def absLong(a: Long): Long
@jvm("$0:D invokestatic java/lang/Math.abs(D)D")
def absDouble(a: Double): Double
@jvm("$0:D invokestatic java/lang/Math.signum(D)D")
def signumDouble(a: Double): Double
@jvm("$0:I $1:I invokestatic java/lang/Math.floorDiv(II)I")
def floorDivInt(a: Int, b: Int): Int
@jvm("$0:J $1:J invokestatic java/lang/Math.floorDiv(JJ)J")
def floorDivLong(a: Long, b: Long): Long
@jvm("$0:I $1:I invokestatic java/lang/Math.floorMod(II)I")
def floorModInt(a: Int, b: Int): Int
@jvm("$0:J $1:J invokestatic java/lang/Math.floorMod(JJ)J")
def floorModLong(a: Long, b: Long): Long

// The generic `max`, `min`, `abs`, `signum`, `floorDiv` and `floorMod` of `Math` and
// `scala.math`: the operands arrive boxed and the result has the class of the wider one.
def numMax(a: Any, b: Any): Any =
  if isBoxedFractional(a) || isBoxedFractional(b) then maxDouble(doubleValue(a), doubleValue(b))
  else if isBoxedLong(a) || isBoxedLong(b) then maxLong(longValue(a), longValue(b))
  else maxInt(intValue(a), intValue(b))

def numMin(a: Any, b: Any): Any =
  if isBoxedFractional(a) || isBoxedFractional(b) then minDouble(doubleValue(a), doubleValue(b))
  else if isBoxedLong(a) || isBoxedLong(b) then minLong(longValue(a), longValue(b))
  else minInt(intValue(a), intValue(b))

def numAbs(a: Any): Any =
  if isBoxedFractional(a) then absDouble(doubleValue(a))
  else if isBoxedLong(a) then absLong(longValue(a))
  else absInt(intValue(a))

def numSignum(a: Any): Any =
  if isBoxedFractional(a) then signumDouble(doubleValue(a))
  else if isBoxedLong(a) then
    val l = longValue(a)
    if l > 0L then 1L else if l < 0L then -1L else 0L
  else
    val i = intValue(a)
    if i > 0 then 1 else if i < 0 then -1 else 0

def numFloorDiv(a: Any, b: Any): Any =
  if isBoxedLong(a) || isBoxedLong(b) then floorDivLong(longValue(a), longValue(b))
  else floorDivInt(intValue(a), intValue(b))

def numFloorMod(a: Any, b: Any): Any =
  if isBoxedLong(a) || isBoxedLong(b) then floorModLong(longValue(a), longValue(b))
  else floorModInt(intValue(a), intValue(b))

// ---- strings ----

@jvm("$0 $1 $2:I invokevirtual java/lang/String.split(Ljava/lang/String;I)[Ljava/lang/String;")
def splitString(s: String, regex: String, limit: Int): Array[String]
@jvm("$0 invokestatic java/util/regex/Pattern.quote(Ljava/lang/String;)Ljava/lang/String;")
def quoteRegex(s: String): String
@jvm("$0:L invokestatic java/lang/String.valueOf(Ljava/lang/Object;)Ljava/lang/String;")
def stringOf(x: Any): String
@jvm("aconst_null")
def nullRef: Any
@jvm("$0 invokestatic java/lang/Long.parseLong(Ljava/lang/String;)J")
def parseLongUnchecked(s: String): Long
@jvm("$0 invokestatic java/lang/Double.parseDouble(Ljava/lang/String;)D")
def parseDoubleUnchecked(s: String): Double
@jvm("$0 $1 invokevirtual java/lang/String.matches(Ljava/lang/String;)Z")
def matchesRegex(s: String, regex: String): Boolean
@jvm("$0 $1 invokestatic java/lang/String.format(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;")
def formatString(format: String, args: Array[Any]): String
@jvm("$0 invokevirtual java/lang/String.translateEscapes()Ljava/lang/String;")
def translateEscapes(s: String): String

// A single character is taken literally, as `split(Char)` needs; a longer separator is a
// regular expression, as in Java.
def separatorRegex(separator: Any): String =
  val text = stringOf(separator)
  if isCharacter(separator) || text.length == 1 then quoteRegex(text) else text

def stripMarginOf(s: String, margin: Char): String =
  val lines = splitString(s, "\n", -1)
  var out = ""
  var i = 0
  while i < lines.length do
    val line = lines(i)
    val at = line.indexOf(margin)
    val stripped = if at >= 0 && line.substring(0, at).isBlank then line.substring(at + 1) else line
    out = if i == 0 then stripped else out + "\n" + stripped
    i += 1
  out

def intOrNull(s: String): Any =
  if !matchesRegex(s, "[+-]?\\d{1,10}") then nullRef
  else
    val l = parseLongUnchecked(s)
    if l > 2147483647L || l < -2147483648L then nullRef else l.toInt

def longOrNull(s: String): Any =
  if !matchesRegex(s, "[+-]?\\d{1,19}") then nullRef
  else if matchesRegex(s, "[+-]?\\d{1,18}") then parseLongUnchecked(s)
  else
    val digits = if s.startsWith("-") || s.startsWith("+") then s.substring(1) else s
    val limit = if s.startsWith("-") then "9223372036854775808" else "9223372036854775807"
    if digits.length == 19 && digits.compareTo(limit) > 0 then nullRef else parseLongUnchecked(s)

def doubleOrNull(s: String): Any =
  if matchesRegex(s, "[\\x00-\\x20]*[+-]?(NaN|Infinity|((\\d+\\.?\\d*|\\.\\d+)([eE][+-]?\\d+)?)[fFdD]?)[\\x00-\\x20]*") then parseDoubleUnchecked(s)
  else nullRef

// f"...": an argument is formatted by the specifier that follows it, or as %s without one.
def formatInterpolatedParts(parts: Array[String], args: Array[Any]): String =
  var format = translateEscapes(parts(0))
  var i = 1
  while i < parts.length do
    val part = translateEscapes(parts(i))
    val specified = matchesRegex(part, "(?s)%[-#+ 0,(]*\\d*(\\.\\d+)?[a-zA-Z].*") && !part.startsWith("%n")
    format = format + (if specified then "" else "%s") + part
    i += 1
  formatString(format, args)

// ---- arrays ----

@jvm("$0 $1 invokevirtual java/util/ArrayList.addAll(Ljava/util/Collection;)Z pop")
def appendAll[A](items: RawBuffer[A], values: RawBuffer[A]): Unit
@jvm("$0 $1:I $2:I invokevirtual java/util/ArrayList.subList(II)Ljava/util/List; invokeinterface java/util/List.clear()V")
def removeRange[A](items: RawBuffer[A], from: Int, until: Int): Unit

// A stable sort, as `Array.prototype.sort` and `java.util.Arrays.sort` of objects are.
def mergeSort[A](arr: RawBuffer[A], compare: (A, A) => Int): Unit =
  val n = arr.length
  if n > 1 then
    var src = copyArray[A, A](arr)
    var dst = copyArray[A, A](arr)
    var width = 1
    while width < n do
      var lo = 0
      while lo < n do
        val mid = minInt(lo + width, n)
        val hi = minInt(lo + 2 * width, n)
        var i = lo
        var j = mid
        var k = lo
        while k < hi do
          if i < mid && (j >= hi || compare(src(i), src(j)) <= 0) then
            dst(k) = src(i)
            i += 1
          else
            dst(k) = src(j)
            j += 1
          k += 1
        lo += 2 * width
      val t = src
      src = dst
      dst = t
      width = width * 2
    var i = 0
    while i < n do
      arr(i) = src(i)
      i += 1

// ---- hashes of collections: scala.util.hashing.MurmurHash3 ----

// Elements whose hashes form an arithmetic progression hash as the Range they could be.
def orderedHash(xs: IterableOnce[Any]): Int =
  val seed = "Seq".hashCode
  var h = seed
  var n = 0
  var prev = 0
  var first = 0
  var step = 0
  var state = 0
  xs.foreach: x =>
    val k = anyHash(x)
    h = mix(h, k)
    if state == 0 then
      first = k
      state = 1
    else if state == 1 then
      step = k - prev
      state = 2
    else if state == 2 && step != k - prev then state = 3
    prev = k
    n += 1
  if state == 2 then avalanche(mix(mix(mix(seed, first), step), prev)) else finalizeHash(h, n)

def unorderedHashOf(a: Int, b: Int, c: Int, n: Int, seed: Int): Int =
  var h = mix(seed, a)
  h = mix(h, b)
  h = mixLast(h, c)
  finalizeHash(h, n)

def unorderedHash(xs: IterableOnce[Any]): Int =
  var a = 0
  var b = 0
  var c = 1
  var n = 0
  xs.foreach: x =>
    val k = anyHash(x)
    a = a + k
    b = b ^ k
    c = c * (k | 1)
    n += 1
  unorderedHashOf(a, b, c, n, "Set".hashCode)

// ---- maps ----

// The equality and MurmurHash3.mapHash of two mutable maps over their LinkedHashMap stores.
@leanOnly def mapEquals(a: Any, b: Any): Boolean =
  val x: RawMap[Any, Any] = unsafeCast(a)
  val y: RawMap[Any, Any] = unsafeCast(b)
  if x.rawSize != y.rawSize then false
  else
    val ks = x.rawKeys
    var same = true
    var i = 0
    while same && i < ks.length do
      same = y.rawHas(ks(i)) && equal(x.rawGet(ks(i)), y.rawGet(ks(i)))
      i += 1
    same

// MurmurHash3.mapHash: the unordered hash of the entries as Tuple2s.
@leanOnly def mapHash(m: Any): Int =
  val x: RawMap[Any, Any] = unsafeCast(m)
  val ks = x.rawKeys
  var a = 0
  var b = 0
  var c = 1
  var i = 0
  while i < ks.length do
    val e = finalizeHash(mix(mix(mix(-889275714, "Tuple2".hashCode), anyHash(ks(i))), anyHash(x.rawGet(ks(i)))), 2)
    a = a + e
    b = b ^ e
    c = c * (e | 1)
    i += 1
  unorderedHashOf(a, b, c, ks.length, "Map".hashCode)

// `()` is JavaScript's `undefined`: what `js.isUndefined` answers for both.
@jvm("$0:L instanceof scala/runtime/BoxedUnit")
def isBoxedUnit(x: Any): Boolean
def isUndefinedValue(x: Any): Boolean = isNull(x) || isBoxedUnit(x)


// ---- regular expressions over java.util.regex; a match is a java.util.regex.MatchResult ----

@jvm("$0 invokestatic java/util/regex/Pattern.compile(Ljava/lang/String;)Ljava/util/regex/Pattern; $1:Ljava/lang/CharSequence; invokevirtual java/util/regex/Pattern.matcher(Ljava/lang/CharSequence;)Ljava/util/regex/Matcher;")
def matcherOf(regex: String, source: String): Any
@jvm("$0:L checkcast java/util/regex/Matcher invokevirtual java/util/regex/Matcher.find()Z")
def matcherFind(m: Any): Boolean
@jvm("$0:L checkcast java/util/regex/Matcher invokevirtual java/util/regex/Matcher.lookingAt()Z")
def matcherLookingAt(m: Any): Boolean
@jvm("$0:L checkcast java/util/regex/Matcher invokevirtual java/util/regex/Matcher.matches()Z")
def matcherMatches(m: Any): Boolean
@jvm("$0:L checkcast java/util/regex/Matcher invokevirtual java/util/regex/Matcher.toMatchResult()Ljava/util/regex/MatchResult;")
def matchResultOf(m: Any): Any
@jvm("new java/lang/StringBuilder dup invokespecial java/lang/StringBuilder.<init>()V")
def newJavaBuilder(): Any
@jvm("$0:L checkcast java/util/regex/Matcher $1:L checkcast java/lang/StringBuilder $2 invokevirtual java/util/regex/Matcher.appendReplacement(Ljava/lang/StringBuilder;Ljava/lang/String;)Ljava/util/regex/Matcher; pop")
def appendReplacement(m: Any, builder: Any, replacement: String): Unit
@jvm("$0:L checkcast java/util/regex/Matcher $1:L checkcast java/lang/StringBuilder invokevirtual java/util/regex/Matcher.appendTail(Ljava/lang/StringBuilder;)Ljava/lang/StringBuilder; invokevirtual java/lang/StringBuilder.toString()Ljava/lang/String;")
def appendTail(m: Any, builder: Any): String
@jvm("$0:L checkcast java/util/regex/MatchResult $1:I invokeinterface java/util/regex/MatchResult.group(I)Ljava/lang/String;")
def groupByIndex(raw: Any, index: Int): String
@jvm("$0:L checkcast java/util/regex/MatchResult $1 invokeinterface java/util/regex/MatchResult.group(Ljava/lang/String;)Ljava/lang/String;")
def groupByName(raw: Any, name: String): String

// ---- arrays of a kind the type does not say ----

// An array whose kind only the value knows, by its kind: the retired lean JVM mode's, which
// the reach still keeps uncalled (calls go to `ScalaRunTime`); removal moves line numbers.
def array_apply(xs: Any, idx: Int): Any = xs match
  case x: Array[AnyRef] => x(idx)
  case x: Array[Int] => x(idx)
  case x: Array[Double] => x(idx)
  case x: Array[Long] => x(idx)
  case x: Array[Float] => x(idx)
  case x: Array[Char] => x(idx)
  case x: Array[Byte] => x(idx)
  case x: Array[Short] => x(idx)
  case x: Array[Boolean] => x(idx)
  case _ => throw new NullPointerException("the array is null")

def array_update(xs: Any, idx: Int, value: Any): Unit = xs match
  case x: Array[AnyRef] => x(idx) = unsafeCast[Any, AnyRef](value)
  case x: Array[Int] => x(idx) = value.asInstanceOf[Int]
  case x: Array[Double] => x(idx) = value.asInstanceOf[Double]
  case x: Array[Long] => x(idx) = value.asInstanceOf[Long]
  case x: Array[Float] => x(idx) = value.asInstanceOf[Float]
  case x: Array[Char] => x(idx) = value.asInstanceOf[Char]
  case x: Array[Byte] => x(idx) = value.asInstanceOf[Byte]
  case x: Array[Short] => x(idx) = value.asInstanceOf[Short]
  case x: Array[Boolean] => x(idx) = value.asInstanceOf[Boolean]
  case _ => throw new NullPointerException("the array is null")

@jvm("$0:L invokestatic java/lang/reflect/Array.getLength(Ljava/lang/Object;)I")
private def lengthOfArray(xs: Any): Int
def array_length(xs: Any): Int = lengthOfArray(xs)

def array_clone(xs: Any): Any = xs match
  case x: Array[AnyRef] => x.clone()
  case x: Array[Int] => x.clone()
  case x: Array[Double] => x.clone()
  case x: Array[Long] => x.clone()
  case x: Array[Float] => x.clone()
  case x: Array[Char] => x.clone()
  case x: Array[Byte] => x.clone()
  case x: Array[Short] => x.clone()
  case x: Array[Boolean] => x.clone()
  case _ => throw new NullPointerException("the array is null")

@jvm("$0:L invokevirtual java/lang/Object.getClass()Ljava/lang/Class; invokevirtual java/lang/Class.isArray()Z")
private def isArrayValue(x: Any): Boolean
/** Whether `x` is an array of arrays `atLevel` deep, as scala-library's `ScalaRunTime.isArray`. */
def isArray(x: Any, atLevel: Int): Boolean =
  !isNull(x) && isArrayValue(x) && (atLevel == 1 || arrayClassDepth(x) >= atLevel)

@jvm("$0:L invokevirtual java/lang/Object.getClass()Ljava/lang/Class; invokevirtual java/lang/Class.getName()Ljava/lang/String;")
private def classNameOfValue(x: Any): String
private def arrayClassDepth(x: Any): Int =
  val name = classNameOfValue(x)
  var depth = 0
  while depth < name.length && name.charAt(depth) == '[' do depth += 1
  depth


// ---- arrays and the std's buffer: an array is made by its kind and filled from a buffer ----

@jvm("$0 $1:I invokestatic java/lang/reflect/Array.newInstance(Ljava/lang/Class;I)Ljava/lang/Object;")
private def newInstanceOf(element: Class[?], length: Int): Any
@jvm("$0:L invokevirtual java/lang/Object.getClass()Ljava/lang/Class;")
private def classOfValue(x: Any): Class[?]
@jvm("$0 invokevirtual java/lang/Class.isPrimitive()Z")
private def isPrimitiveClass(c: Class[?]): Boolean

// The class of the elements a tag's arrays hold: an `Array[Unit]` holds boxed units.
private def elementClassOf(tag: scala.reflect.ClassTag[?]): Class[?] =
  val c = tag.runtimeClass
  if isPrimitiveClass(c) && c.getName == "void" then classOfValue(()) else c

def newArrayOfTag[T](tag: scala.reflect.ClassTag[T], length: Int): Array[T] =
  unsafeCast(newInstanceOf(elementClassOf(tag), length))

def wrappedTag[T](tag: scala.reflect.ClassTag[T]): scala.reflect.ClassTag[Array[T]] =
  new scala.reflect.ClassTag(classOfValue(newInstanceOf(elementClassOf(tag), 0)))

private def fillArray[T](out: Array[T], b: RawBuffer[T]): Array[T] =
  var i = 0
  while i < b.length do
    out(i) = b(i)
    i += 1
  out

def arrayOfTag[T](b: RawBuffer[T], tag: scala.reflect.ClassTag[T]): Array[T] = fillArray(newArrayOfTag(tag, b.length), b)

def arrayLikeOf[T](b: RawBuffer[T], other: Any): Array[T] = fillArray(unsafeCast[Any, Array[T]](newArrayLike(other, b.length)), b)

@leanOnly def taggedBuilder[T](tag: scala.reflect.ClassTag[T]): Any = new TaggedArrayBuilder[T](tag)

def bufferOfArray[T](a: Array[T]): RawBuffer[T] =
  val n = a.length
  val out = bufferOfCapacity[T](n)
  var i = 0
  while i < n do
    out.push(a(i))
    i += 1
  out

/** `xs.toArray` of the lean std's collections: the elements in an array of the tag's kind. */
@leanOnly def arrayOf[A](xs: IterableOnce[A], tag: scala.reflect.ClassTag[A]): Array[A] = arrayOfTag(rawItems(xs), tag)

// ---- scala.StringBuilder in the lean mode, the JDK's class: the members that class lacks ----

def builderTake(sb: java.lang.StringBuilder, n: Int): java.lang.StringBuilder = new java.lang.StringBuilder(sb.toString.take(n))
def builderDrop(sb: java.lang.StringBuilder, n: Int): java.lang.StringBuilder = new java.lang.StringBuilder(sb.toString.drop(n))
def builderTakeRight(sb: java.lang.StringBuilder, n: Int): java.lang.StringBuilder = new java.lang.StringBuilder(sb.toString.takeRight(n))
def builderDropRight(sb: java.lang.StringBuilder, n: Int): java.lang.StringBuilder = new java.lang.StringBuilder(sb.toString.dropRight(n))
def builderContains(sb: java.lang.StringBuilder, elem: Any): Boolean = elem match
  case c: Char => sb.indexOf(c.toString) >= 0
  case _ => false
def builderForeach(sb: java.lang.StringBuilder, f: Char => Any): Unit = sb.toString.foreach(f)
def builderToList(sb: java.lang.StringBuilder): List[Char] = sb.toString.toList

// mode: "" first match, "y" match at the start, "f" match of the whole input.
def regexExec(regex: String, mode: String, source: String): Any =
  val m = matcherOf(regex, source)
  val found = if mode == "f" then matcherMatches(m) else if mode == "y" then matcherLookingAt(m) else matcherFind(m)
  if found then matchResultOf(m) else nullRef

def regexAll(regex: String, source: String): Array[Any] =
  val counted = matcherOf(regex, source)
  var n = 0
  while matcherFind(counted) do n += 1
  val out = new Array[Any](n)
  val m = matcherOf(regex, source)
  var i = 0
  while matcherFind(m) do
    out(i) = matchResultOf(m)
    i += 1
  out

// `replacement` is a replacement string or a function from the wrapped match to one.
def regexReplace(regex: String, target: String, replacement: Any, all: Boolean, wrap: Any => Any): String =
  val m = matcherOf(regex, target)
  val builder = newJavaBuilder()
  var more = true
  while more && matcherFind(m) do
    val text: String = replacement match
      case s: String => s
      case f: (Any => Any) @unchecked => stringOf(f(wrap(matchResultOf(m))))
    appendReplacement(m, builder, text)
    more = all
  appendTail(m, builder)

def regexSplit(source: String, regex: String, limit: Int): Array[String] = splitString(source, regex, limit)

def regexGroup(raw: Any, id: Any): String =
  if isBoxedInt(id) then groupByIndex(raw, intValue(id)) else groupByName(raw, stringOf(id))

// A key of a hash store that compares as `==` does: 1, 1.0, 1L and 'a' find each other's
// entries, and 0.0 finds -0.0. Everything that is no number is its own key.
final class NumKey(val value: Any):
  override def equals(that: Any): Boolean = that match
    case k: NumKey => equal(value, k.value)
    case _ => equal(value, that)
  override def hashCode: Int = anyHash(value)
  override def toString: String = stringOf(value)

def wrapKey(k: Any): Any = if isNumber(k) || isCharacter(k) then new NumKey(k) else k

def unwrapKeys(keys: RawBuffer[Any]): RawBuffer[Any] =
  var i = 0
  while i < keys.length do
    keys(i) match
      case k: NumKey => keys(i) = k.value
      case _ => ()
    i += 1
  keys

// A JDK method's `byte[]` as this target's array of bytes and back, through their hex text.
@jvm("invokestatic java/util/HexFormat.of()Ljava/util/HexFormat; $0:L checkcast [B invokevirtual java/util/HexFormat.formatHex([B)Ljava/lang/String;")
def hexOfJavaBytes(a: Any): String
@jvm("invokestatic java/util/HexFormat.of()Ljava/util/HexFormat; $0 invokevirtual java/util/HexFormat.parseHex(Ljava/lang/CharSequence;)[B")
def javaBytesOfHex(hex: String): Any
def bytesOfJava(a: Any): Any =
  val hex = hexOfJavaBytes(a)
  Array.tabulate[Byte](hex.length / 2)(i => java.lang.Integer.parseInt(hex.substring(2 * i, 2 * i + 2), 16).toByte)
def javaBytes(a: Any): Any =
  val bytes = a.asInstanceOf[Array[Byte]]
  val hex = new java.lang.StringBuilder()
  for b <- bytes do
    hex.append("0123456789abcdef".charAt((b >> 4) & 15))
    hex.append("0123456789abcdef".charAt(b & 15))
  javaBytesOfHex(hex.toString)

// java.util.Arrays and System.arraycopy of the platform layer: a copy is of the array's own kind.
@jvm("$0:L invokestatic java/lang/reflect/Array.getLength(Ljava/lang/Object;)I")
private def reflectedLength(a: Any): Int
@jvm("$0:L invokevirtual java/lang/Object.getClass()Ljava/lang/Class; invokevirtual java/lang/Class.getComponentType()Ljava/lang/Class; $1:I invokestatic java/lang/reflect/Array.newInstance(Ljava/lang/Class;I)Ljava/lang/Object;")
private def newArrayLike(a: Any, length: Int): Any
@jvm("$0:L $1:I $2:L $3:I $4:I invokestatic java/lang/System.arraycopy(Ljava/lang/Object;ILjava/lang/Object;II)V")
private def systemArraycopy(src: Any, srcPos: Int, dest: Any, destPos: Int, length: Int): Unit
def linkedArrayCopyOf(a: Any, length: Int): Any =
  if length < 0 then throw new NegativeArraySizeException(length.toString)
  linkedArrayCopyOfRange(a, 0, length)
def linkedArrayCopyOfRange(a: Any, from: Int, to: Int): Any =
  val available = reflectedLength(a)
  if from > to then throw new IllegalArgumentException(s"$from > $to")
  if from < 0 || from > available then throw new ArrayIndexOutOfBoundsException(s"Array index out of range: $from")
  val out = newArrayLike(a, to - from)
  val end = if to < available then to else available
  if end > from then systemArraycopy(a, from, out, 0, end - from)
  out
// The members of `java.util.Arrays` that take an array of any kind: the elements are read and
// written by the array's kind, and a primitive array is sorted by the JDK.
def arraysFillAll[T](a: Array[T], value: T): Unit = arraysFill(a, 0, a.length, value)
def arraysFill[T](a: Array[T], from: Int, to: Int, value: T): Unit =
  if from > to then throw new IllegalArgumentException(s"fromIndex($from) > toIndex($to)")
  var i = from
  while i < to do
    a(i) = value
    i += 1
@jvm("$0 $1:I $2:I invokestatic java/util/Arrays.sort([III)V")
private def sortInts(a: Array[Int], from: Int, to: Int): Unit
@jvm("$0 $1:I $2:I invokestatic java/util/Arrays.sort([JII)V")
private def sortLongs(a: Array[Long], from: Int, to: Int): Unit
@jvm("$0 $1:I $2:I invokestatic java/util/Arrays.sort([DII)V")
private def sortDoubles(a: Array[Double], from: Int, to: Int): Unit
@jvm("$0 $1:I $2:I invokestatic java/util/Arrays.sort([FII)V")
private def sortFloats(a: Array[Float], from: Int, to: Int): Unit
@jvm("$0 $1:I $2:I invokestatic java/util/Arrays.sort([CII)V")
private def sortChars(a: Array[Char], from: Int, to: Int): Unit
@jvm("$0 $1:I $2:I invokestatic java/util/Arrays.sort([BII)V")
private def sortBytes(a: Array[Byte], from: Int, to: Int): Unit
@jvm("$0 $1:I $2:I invokestatic java/util/Arrays.sort([SII)V")
private def sortShorts(a: Array[Short], from: Int, to: Int): Unit
def arraysSortNatural[T](a: Array[T]): Unit = arraysSortRange(a, 0, a.length)
def arraysSort[T](a: Array[T], comparator: Any): Unit = arraysSortRangeWith(a, 0, a.length, comparator)
def arraysSortRange[T](a: Array[T], from: Int, to: Int): Unit = (a: Any) match
  case x: Array[Int] => sortInts(x, from, to)
  case x: Array[Long] => sortLongs(x, from, to)
  case x: Array[Double] => sortDoubles(x, from, to)
  case x: Array[Float] => sortFloats(x, from, to)
  case x: Array[Char] => sortChars(x, from, to)
  case x: Array[Byte] => sortBytes(x, from, to)
  case x: Array[Short] => sortShorts(x, from, to)
  case _ => arraysSortWith(a, from, to, (x, y) => x.asInstanceOf[Comparable[Any]].compareTo(y))
def arraysSortRangeWith[T](a: Array[T], from: Int, to: Int, comparator: Any): Unit =
  if isNull(comparator) then arraysSortRange(a, from, to)
  else
    val cmp = comparator.asInstanceOf[java.util.Comparator[Any]]
    arraysSortWith(a, from, to, (x, y) => cmp.compare(x, y))
private def arraysSortWith[T](a: Array[T], from: Int, to: Int, compare: (Any, Any) => Int): Unit =
  if from > to then throw new IllegalArgumentException(s"fromIndex($from) > toIndex($to)")
  if from < 0 then throw new ArrayIndexOutOfBoundsException(s"Array index out of range: $from")
  if to > a.length then throw new ArrayIndexOutOfBoundsException(s"Array index out of range: $to")
  val part = emptyBuffer[T]
  var i = from
  while i < to do
    part.push(a(i))
    i += 1
  mergeSort[T](part, compare)
  i = from
  while i < to do
    a(i) = part(i - from)
    i += 1
def arraysEquals[T](a: Array[T], b: Array[T]): Boolean =
  if a eq b then true
  else if isNull(a) || isNull(b) then false
  else a.length == b.length && a.sameElements(b.toSeq)
def arraysHashCode[T](a: Array[T]): Int =
  if isNull(a) then 0
  else a.foldLeft(1)((h, e) => 31 * h + (if isNull(e) then 0 else e.hashCode))
def arraysAsList(xs: Any): Any =
  val out = new java.util.ArrayList[Any]()
  xs.asInstanceOf[Seq[Any]].foreach(x => out.add(x))
  out
def collectionsAddAll(c: Any, xs: Any): Boolean =
  var changed = false
  xs.asInstanceOf[Seq[Any]].foreach(x => if c.asInstanceOf[java.util.Collection[Any]].add(x) then changed = true)
  changed
def arraysBinarySearch[T](a: Array[T], key: Any): Int =
  var lo = 0
  var hi = a.length - 1
  while lo <= hi do
    val mid = (lo + hi) >>> 1
    val c = a(mid).asInstanceOf[Comparable[Any]].compareTo(key)
    if c < 0 then lo = mid + 1
    else if c > 0 then hi = mid - 1
    else return mid
  -(lo + 1)
def arraysToString[T](a: Array[T]): String =
  if isNull(a) then "null" else a.mkString("[", ", ", "]")
