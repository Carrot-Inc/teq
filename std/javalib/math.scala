// The Java arithmetic classes for JavaScript: `BigInteger` over the JS BigInt, `BigDecimal` as
// an unscaled `BigInteger` with a scale, the way the JDK defines them, with the JDK's text
// forms, hashes and rounding. On the JVM the classes are the JDK's own (`@jvmClass`), whose
// methods of the same descriptors the members call; the interpreter has the `BigInt`
// operations as builtins (`src/interp/bignum.rs`).
package java.math:

  @js("BigInt($0)")
  def bigOf(digits: String): Any
  @js("$0")
  def bigOfLong(l: Long): Any
  @js("BigInt.asIntN(64, $0)")
  def bigToLong(a: Any): Long
  @js("Number($0)")
  def bigToDouble(a: Any): Double
  @js("($0 + $1)")
  def bigAdd(a: Any, b: Any): Any
  @js("($0 - $1)")
  def bigSub(a: Any, b: Any): Any
  @js("($0 * $1)")
  def bigMul(a: Any, b: Any): Any
  @js("($0 / $1)")
  def bigQuot(a: Any, b: Any): Any
  @js("($0 % $1)")
  def bigRem(a: Any, b: Any): Any
  @js("(-$0)")
  def bigNeg(a: Any): Any
  @js("($0 ** BigInt($1))")
  def bigPow(a: Any, n: Int): Any
  @js("($0 << BigInt($1))")
  def bigShl(a: Any, n: Int): Any
  @js("($0 >> BigInt($1))")
  def bigShr(a: Any, n: Int): Any
  @js("($0 & $1)")
  def bigAnd(a: Any, b: Any): Any
  @js("($0 | $1)")
  def bigOr(a: Any, b: Any): Any
  @js("($0 ^ $1)")
  def bigXor(a: Any, b: Any): Any
  @js("(~$0)")
  def bigNot(a: Any): Any
  @js("($0 < $1 ? -1 : $0 > $1 ? 1 : 0)")
  def bigCompare(a: Any, b: Any): Int
  @js("($0 === $1)")
  def bigSame(a: Any, b: Any): Boolean
  @js("($0).toString($1)")
  def bigText(a: Any, radix: Int): String
  // The JDK's hash: over the 32-bit words of the magnitude, most significant first, times the sign.
  @js("((a) => { let m = a < 0n ? -a : a; const w = []; while (m > 0n) { w.push(Number(m & 0xffffffffn)); m >>= 32n; } let h = 0; for (let i = w.length - 1; i >= 0; i--) h = (Math.imul(31, h) + w[i]) | 0; return a < 0n ? (-h) | 0 : h; })($0)")
  def bigHash(a: Any): Int
  @js("Number($0)")
  def doubleOfText(text: String): Double
  @js("((a) => { if (a === 0n) return -1; let n = 0; while ((a & 1n) === 0n) { a >>= 1n; n++; } return n; })($0)")
  def bigLowestSetBit(a: Any): Int

  def bigZero: Any = bigOfLong(0L)
  def bigTen: Any = bigOfLong(10L)
  def bigSignum(a: Any): Int = bigCompare(a, bigZero)
  def bigAbs(a: Any): Any = if bigSignum(a) < 0 then bigNeg(a) else a
  def bigIsZero(a: Any): Boolean = bigSignum(a) == 0
  def bigIsOdd(a: Any): Boolean = !bigIsZero(bigAnd(a, bigOfLong(1L)))
  def tenTo(n: Int): Any = bigPow(bigTen, n)
  def digitsOf(a: Any): Int = bigText(bigAbs(a), 10).length
  // The bits of the two's complement below the sign bit, as the JDK counts them.
  def bigBitLength(a: Any): Int =
    val m = if bigSignum(a) < 0 then bigSub(bigNeg(a), bigOfLong(1L)) else a
    if bigIsZero(m) then 0 else bigText(m, 2).length
  def bigGcd(a: Any, b: Any): Any =
    var x = bigAbs(a)
    var y = bigAbs(b)
    while !bigIsZero(y) do
      val r = bigRem(x, y)
      x = y
      y = r
    x

  def bigOne: Any = bigOfLong(1L)
  def bigBit(n: Int): Any =
    if n < 0 then throw new java.lang.ArithmeticException("Negative bit address")
    bigShl(bigOne, n)
  // The two's complement bytes, most significant first; the magnitude's when `unsigned`.
  def bigOfBytes(bytes: Array[Byte], unsigned: Boolean): Any =
    var acc = bigZero
    var i = 0
    while i < bytes.length do
      acc = bigOr(bigShl(acc, 8), bigOfLong((bytes(i) & 255).toLong))
      i += 1
    if !unsigned && bytes.length > 0 && bytes(0) < 0 then bigSub(acc, bigShl(bigOne, 8 * bytes.length)) else acc
  def bigMod(a: Any, m: Any): Any =
    val r = bigRem(a, m)
    if bigSignum(r) < 0 then bigAdd(r, m) else r
  def bigModPow(base: Any, exponent: Any, m: Any): Any =
    var result = bigMod(bigOne, m)
    var b = bigMod(base, m)
    var e = exponent
    while !bigIsZero(e) do
      if bigIsOdd(e) then result = bigRem(bigMul(result, b), m)
      b = bigRem(bigMul(b, b), m)
      e = bigShr(e, 1)
    result
  // Miller-Rabin over the first twelve primes, exact below 3.3e24, and a few more bases above.
  def bigIsProbablePrime(n: Any): Boolean =
    val two = bigOfLong(2L)
    if bigCompare(n, two) < 0 then return false
    if bigSame(n, two) || bigSame(n, bigOfLong(3L)) then return true
    if !bigIsOdd(n) then return false
    val nm1 = bigSub(n, bigOne)
    var d = nm1
    var s = 0
    while !bigIsOdd(d) do
      d = bigShr(d, 1)
      s += 1
    val bases = Array(2L, 3L, 5L, 7L, 11L, 13L, 17L, 19L, 23L, 29L, 31L, 37L, 41L, 43L, 47L, 53L, 59L, 61L, 67L, 71L)
    var i = 0
    while i < bases.length do
      val a = bigOfLong(bases(i))
      if bigCompare(a, nm1) < 0 then
        var x = bigModPow(a, d, n)
        if !bigSame(x, bigOne) && !bigSame(x, nm1) then
          var r = 1
          var witness = true
          while r < s && witness do
            x = bigRem(bigMul(x, x), n)
            if bigSame(x, nm1) then witness = false
            r += 1
          if witness then return false
      i += 1
    true

  def digitValue(c: Char, radix: Int): Int =
    val d =
      if c >= '0' && c <= '9' then c - '0'
      else if c >= 'a' && c <= 'z' then c - 'a' + 10
      else if c >= 'A' && c <= 'Z' then c - 'A' + 10
      else -1
    if d < radix then d else -1

  def parseBig(text: String, radix: Int): Any =
    val negative = text.startsWith("-")
    val start = if negative || text.startsWith("+") then 1 else 0
    if text.length == start then throw new java.lang.NumberFormatException("Zero length BigInteger")
    if radix == 10 then
      var i = start
      while i < text.length do
        if digitValue(text.charAt(i), 10) < 0 then throw new java.lang.NumberFormatException("For input string: \"" + text + "\"")
        i += 1
      bigOf(text)
    else
      var acc = bigZero
      var i = start
      while i < text.length do
        val d = digitValue(text.charAt(i), radix)
        if d < 0 then throw new java.lang.NumberFormatException("For input string: \"" + text + "\"")
        acc = bigAdd(bigMul(acc, bigOfLong(radix.toLong)), bigOfLong(d.toLong))
        i += 1
      if negative then bigNeg(acc) else acc

  // `num / den` rounded as the mode says, for a positive `den`.
  def divideRounded(num: Any, den: Any, mode: RoundingMode): Any =
    val q = bigQuot(num, den)
    val r = bigRem(num, den)
    if bigIsZero(r) then return q
    val sign = bigSignum(num)
    val twice = bigCompare(bigMul(bigAbs(r), bigOfLong(2L)), den)
    val away = mode.position match
      case 0 => true
      case 1 => false
      case 2 => sign > 0
      case 3 => sign < 0
      case 4 => twice >= 0
      case 5 => twice > 0
      case 6 => twice > 0 || (twice == 0 && bigIsOdd(q))
      case _ => throw new java.lang.ArithmeticException("Rounding necessary")
    if away then bigAdd(q, bigOfLong(sign.toLong)) else q

  @javaDefined
  @jvmClass("java/math/RoundingMode")
  final class RoundingMode(val label: String, val position: Int):
    @javaDefined
    @javaDefined
    def name: String = label
    @javaDefined
    @javaDefined
    def ordinal: Int = position
    override def toString: String = label
    override def hashCode: Int = label.hashCode
    override def equals(that: Any): Boolean = that match
      case m: RoundingMode => m.position == position
      case _ => false

  @javaDefined
  @jvmClass("java/math/RoundingMode")
  object RoundingMode:
    private val upValue: RoundingMode = new RoundingMode("UP", 0)
    @jvm("getstatic java/math/RoundingMode.UP:Ljava/math/RoundingMode;")
    def UP: RoundingMode = upValue
    private val downValue: RoundingMode = new RoundingMode("DOWN", 1)
    @jvm("getstatic java/math/RoundingMode.DOWN:Ljava/math/RoundingMode;")
    def DOWN: RoundingMode = downValue
    private val ceilingValue: RoundingMode = new RoundingMode("CEILING", 2)
    @jvm("getstatic java/math/RoundingMode.CEILING:Ljava/math/RoundingMode;")
    def CEILING: RoundingMode = ceilingValue
    private val floorValue: RoundingMode = new RoundingMode("FLOOR", 3)
    @jvm("getstatic java/math/RoundingMode.FLOOR:Ljava/math/RoundingMode;")
    def FLOOR: RoundingMode = floorValue
    private val halfUpValue: RoundingMode = new RoundingMode("HALF_UP", 4)
    @jvm("getstatic java/math/RoundingMode.HALF_UP:Ljava/math/RoundingMode;")
    def HALF_UP: RoundingMode = halfUpValue
    private val halfDownValue: RoundingMode = new RoundingMode("HALF_DOWN", 5)
    @jvm("getstatic java/math/RoundingMode.HALF_DOWN:Ljava/math/RoundingMode;")
    def HALF_DOWN: RoundingMode = halfDownValue
    private val halfEvenValue: RoundingMode = new RoundingMode("HALF_EVEN", 6)
    @jvm("getstatic java/math/RoundingMode.HALF_EVEN:Ljava/math/RoundingMode;")
    def HALF_EVEN: RoundingMode = halfEvenValue
    private val unnecessaryValue: RoundingMode = new RoundingMode("UNNECESSARY", 7)
    @jvm("getstatic java/math/RoundingMode.UNNECESSARY:Ljava/math/RoundingMode;")
    def UNNECESSARY: RoundingMode = unnecessaryValue
    @javaDefined
    @jvm("invokestatic java/math/RoundingMode.values()[Ljava/math/RoundingMode;")
    def values: Array[RoundingMode] = Array(UP, DOWN, CEILING, FLOOR, HALF_UP, HALF_DOWN, HALF_EVEN, UNNECESSARY)
    @jvm("invokestatic java/math/RoundingMode.valueOf(I)Ljava/math/RoundingMode;")
    def valueOf(mode: Int): RoundingMode =
      if mode < 0 || mode > 7 then throw new java.lang.IllegalArgumentException("argument out of range")
      values(mode)
    @jvm("invokestatic java/math/RoundingMode.valueOf(Ljava/lang/String;)Ljava/math/RoundingMode;")
    def valueOf(name: String): RoundingMode =
      val all = values
      var i = 0
      while i < all.length do
        if all(i).label == name then return all(i)
        i += 1
      throw new java.lang.IllegalArgumentException("No enum constant java.math.RoundingMode." + name)

  @javaDefined
  @jvmClass("java/math/MathContext")
  final class MathContext(precisionValue: Int, mode: RoundingMode):
    if precisionValue < 0 then throw new java.lang.IllegalArgumentException("Digits < 0")
    def this(precision: Int) = this(precision, RoundingMode.HALF_UP)
    @javaDefined
    def getPrecision: Int = precisionValue
    @javaDefined
    def getRoundingMode: RoundingMode = mode
    override def equals(that: Any): Boolean = that match
      case mc: MathContext => mc.getPrecision == precisionValue && mc.getRoundingMode == mode
      case _ => false
    override def hashCode: Int = precisionValue + mode.hashCode * 59
    override def toString: String = "precision=" + precisionValue + " roundingMode=" + mode

  @javaDefined
  @jvmClass("java/math/MathContext")
  object MathContext:
    private val unlimitedValue: MathContext = new MathContext(0, RoundingMode.HALF_UP)
    @jvm("getstatic java/math/MathContext.UNLIMITED:Ljava/math/MathContext;")
    def UNLIMITED: MathContext = unlimitedValue
    private val decimal32Value: MathContext = new MathContext(7, RoundingMode.HALF_EVEN)
    @jvm("getstatic java/math/MathContext.DECIMAL32:Ljava/math/MathContext;")
    def DECIMAL32: MathContext = decimal32Value
    private val decimal64Value: MathContext = new MathContext(16, RoundingMode.HALF_EVEN)
    @jvm("getstatic java/math/MathContext.DECIMAL64:Ljava/math/MathContext;")
    def DECIMAL64: MathContext = decimal64Value
    private val decimal128Value: MathContext = new MathContext(34, RoundingMode.HALF_EVEN)
    @jvm("getstatic java/math/MathContext.DECIMAL128:Ljava/math/MathContext;")
    def DECIMAL128: MathContext = decimal128Value

  @javaDefined
  @jvmClass("java/math/BigInteger")
  final class BigInteger(val bits: Any) extends java.lang.Number, java.lang.Comparable[BigInteger]:
    def this(text: String) = this(parseBig(text, 10))
    def this(text: String, radix: Int) = this(parseBig(text, radix))
    def this(bytes: Array[Byte]) = this(
      if bytes.length == 0 then throw new java.lang.NumberFormatException("Zero length BigInteger") else bigOfBytes(bytes, false))
    def this(signum: Int, magnitude: Array[Byte]) = this({
      if signum < -1 || signum > 1 then throw new java.lang.NumberFormatException("Invalid signum value")
      val m = bigOfBytes(magnitude, true)
      if signum == 0 && !bigIsZero(m) then throw new java.lang.NumberFormatException("signum-magnitude mismatch")
      if signum < 0 then bigNeg(m) else m
    })
    def add(that: BigInteger): BigInteger = new BigInteger(bigAdd(bits, that.bits))
    def subtract(that: BigInteger): BigInteger = new BigInteger(bigSub(bits, that.bits))
    def multiply(that: BigInteger): BigInteger = new BigInteger(bigMul(bits, that.bits))
    def divide(that: BigInteger): BigInteger =
      if that.signum == 0 then throw new java.lang.ArithmeticException("BigInteger divide by zero")
      new BigInteger(bigQuot(bits, that.bits))
    def remainder(that: BigInteger): BigInteger =
      if that.signum == 0 then throw new java.lang.ArithmeticException("BigInteger divide by zero")
      new BigInteger(bigRem(bits, that.bits))
    def mod(m: BigInteger): BigInteger =
      if m.signum <= 0 then throw new java.lang.ArithmeticException("BigInteger: modulus not positive")
      val r = bigRem(bits, m.bits)
      new BigInteger(if bigSignum(r) < 0 then bigAdd(r, m.bits) else r)
    def gcd(that: BigInteger): BigInteger = new BigInteger(bigGcd(bits, that.bits))
    @jvm("$0 $1 invokevirtual java/math/BigInteger.divideAndRemainder(Ljava/math/BigInteger;)[Ljava/math/BigInteger;")
    def divideAndRemainder(that: BigInteger): Array[BigInteger] = Array(divide(that), remainder(that))
    def modPow(exponent: BigInteger, m: BigInteger): BigInteger =
      if m.signum <= 0 then throw new java.lang.ArithmeticException("BigInteger: modulus not positive")
      if exponent.signum < 0 then modInverse(m).modPow(exponent.negate, m)
      else new BigInteger(bigModPow(bits, exponent.bits, m.bits))
    def modInverse(m: BigInteger): BigInteger =
      if m.signum <= 0 then throw new java.lang.ArithmeticException("BigInteger: modulus not positive")
      var r0 = m.bits
      var r1 = bigMod(bits, m.bits)
      var t0 = bigZero
      var t1 = bigOne
      while !bigIsZero(r1) do
        val q = bigQuot(r0, r1)
        val r2 = bigSub(r0, bigMul(q, r1))
        r0 = r1
        r1 = r2
        val t2 = bigSub(t0, bigMul(q, t1))
        t0 = t1
        t1 = t2
      if bigSame(m.bits, bigOne) then return BigInteger.ZERO
      if !bigSame(r0, bigOne) then throw new java.lang.ArithmeticException("BigInteger not invertible.")
      new BigInteger(bigMod(t0, m.bits))
    def isProbablePrime(certainty: Int): Boolean = certainty <= 0 || bigIsProbablePrime(bigAbs(bits))
    def setBit(n: Int): BigInteger = new BigInteger(bigOr(bits, bigBit(n)))
    def clearBit(n: Int): BigInteger = new BigInteger(bigAnd(bits, bigNot(bigBit(n))))
    def flipBit(n: Int): BigInteger = new BigInteger(bigXor(bits, bigBit(n)))
    // The bits of the two's complement that differ from the sign bit.
    @javaDefined
    def bitCount: Int =
      val m = if signum < 0 then bigNot(bits) else bits
      val text = bigText(m, 2)
      var n = 0
      var i = 0
      while i < text.length do
        if text.charAt(i) == '1' then n += 1
        i += 1
      n
    @javaDefined
    def toByteArray: Array[Byte] =
      val n = bitLength / 8 + 1
      val out = new Array[Byte](n)
      var i = 0
      while i < n do
        out(n - 1 - i) = bigToLong(bigAnd(bigShr(bits, 8 * i), bigOfLong(255L))).toByte
        i += 1
      out
    @javaDefined
    def negate: BigInteger = new BigInteger(bigNeg(bits))
    @javaDefined
    def abs: BigInteger = if signum < 0 then negate else this
    @javaDefined
    def signum: Int = bigSignum(bits)
    def pow(exponent: Int): BigInteger =
      if exponent < 0 then throw new java.lang.ArithmeticException("Negative exponent")
      new BigInteger(bigPow(bits, exponent))
    def shiftLeft(n: Int): BigInteger = if n >= 0 then new BigInteger(bigShl(bits, n)) else new BigInteger(bigShr(bits, -n))
    def shiftRight(n: Int): BigInteger = if n >= 0 then new BigInteger(bigShr(bits, n)) else new BigInteger(bigShl(bits, -n))
    def and(that: BigInteger): BigInteger = new BigInteger(bigAnd(bits, that.bits))
    def or(that: BigInteger): BigInteger = new BigInteger(bigOr(bits, that.bits))
    def xor(that: BigInteger): BigInteger = new BigInteger(bigXor(bits, that.bits))
    @javaDefined
    def not: BigInteger = new BigInteger(bigNot(bits))
    def andNot(that: BigInteger): BigInteger = and(that.not)
    def testBit(n: Int): Boolean =
      if n < 0 then throw new java.lang.ArithmeticException("Negative bit address")
      bigIsOdd(bigShr(bits, n))
    @javaDefined
    def bitLength: Int = bigBitLength(bits)
    @javaDefined
    def getLowestSetBit: Int = bigLowestSetBit(bits)
    def min(that: BigInteger): BigInteger = if compareTo(that) <= 0 then this else that
    def max(that: BigInteger): BigInteger = if compareTo(that) >= 0 then this else that
    def compareTo(that: BigInteger): Int = bigCompare(bits, that.bits)
    override def equals(that: Any): Boolean = that match
      case b: BigInteger => bigSame(bits, b.bits)
      case _ => false
    override def hashCode: Int = bigHash(bits)
    override def toString: String = bigText(bits, 10)
    def toString(radix: Int): String = bigText(bits, radix)
    @javaDefined
    def intValue: Int = bigToLong(bits).toInt
    @javaDefined
    def longValue: Long = bigToLong(bits)
    @javaDefined
    def doubleValue: Double = bigToDouble(bits)
    @javaDefined
    def floatValue: Float = bigToDouble(bits).toFloat
    @javaDefined
    def longValueExact: Long =
      if bitLength > 63 then throw new java.lang.ArithmeticException("BigInteger out of long range")
      longValue
    @javaDefined
    def intValueExact: Int =
      if bitLength > 31 then throw new java.lang.ArithmeticException("BigInteger out of int range")
      intValue
    @javaDefined
    def shortValueExact: Short =
      if bitLength > 15 then throw new java.lang.ArithmeticException("BigInteger out of short range")
      intValue.toShort
    @javaDefined
    def byteValueExact: Byte =
      if bitLength > 7 then throw new java.lang.ArithmeticException("BigInteger out of byte range")
      intValue.toByte

  @javaDefined
  @jvmClass("java/math/BigInteger")
  object BigInteger:
    private val zeroValue: BigInteger = new BigInteger(bigOfLong(0L))
    @jvm("getstatic java/math/BigInteger.ZERO:Ljava/math/BigInteger;")
    def ZERO: BigInteger = zeroValue
    private val oneValue: BigInteger = new BigInteger(bigOfLong(1L))
    @jvm("getstatic java/math/BigInteger.ONE:Ljava/math/BigInteger;")
    def ONE: BigInteger = oneValue
    private val twoValue: BigInteger = new BigInteger(bigOfLong(2L))
    @jvm("getstatic java/math/BigInteger.TWO:Ljava/math/BigInteger;")
    def TWO: BigInteger = twoValue
    private val tenValue: BigInteger = new BigInteger(bigOfLong(10L))
    @jvm("getstatic java/math/BigInteger.TEN:Ljava/math/BigInteger;")
    def TEN: BigInteger = tenValue
    @jvm("invokestatic java/math/BigInteger.valueOf(J)Ljava/math/BigInteger;")
    def valueOf(l: Long): BigInteger = new BigInteger(bigOfLong(l))

  // `[sign] digits [. digits] [E [sign] digits]`: the digits are the unscaled value, the
  // fraction's length less the exponent is the scale.
  def parseDecimal(text: String): BigDecimal =
    def malformed(): Nothing = throw new java.lang.NumberFormatException("Character array is missing \"exponent\" mark 'e' or 'E'.")
    val negative = text.startsWith("-")
    var i = if negative || text.startsWith("+") then 1 else 0
    var digits = ""
    var scale = 0
    var seen = 0
    while i < text.length && text.charAt(i) >= '0' && text.charAt(i) <= '9' do
      digits = digits + text.charAt(i)
      seen += 1
      i += 1
    if i < text.length && text.charAt(i) == '.' then
      i += 1
      while i < text.length && text.charAt(i) >= '0' && text.charAt(i) <= '9' do
        digits = digits + text.charAt(i)
        scale += 1
        seen += 1
        i += 1
    if seen == 0 then throw new java.lang.NumberFormatException("No digits found.")
    if i < text.length && (text.charAt(i) == 'e' || text.charAt(i) == 'E') then
      i += 1
      val expNegative = i < text.length && text.charAt(i) == '-'
      if i < text.length && (text.charAt(i) == '-' || text.charAt(i) == '+') then i += 1
      var exp = 0
      var expDigits = 0
      while i < text.length && text.charAt(i) >= '0' && text.charAt(i) <= '9' do
        exp = exp * 10 + (text.charAt(i) - '0')
        expDigits += 1
        i += 1
      if expDigits == 0 then malformed()
      scale = if expNegative then scale + exp else scale - exp
    if i != text.length then malformed()
    val unscaled = bigOf(digits)
    new BigDecimal(new BigInteger(if negative then bigNeg(unscaled) else unscaled), scale)

  // The exact value of a double: its significand times a power of two, as a decimal.
  def decimalOfDouble(d: Double): BigDecimal =
    if java.lang.Double.isNaN(d) || java.lang.Double.isInfinite(d) then throw new java.lang.NumberFormatException("Infinite or NaN")
    if d == 0.0 then return BigDecimal.ZERO
    val bits = java.lang.Double.doubleToLongBits(d)
    val biased = ((bits >> 52) & 0x7ffL).toInt
    val fraction = bits & 0xfffffffffffffL
    var significand = if biased == 0 then fraction else fraction | 0x10000000000000L
    var exponent = if biased == 0 then -1074 else biased - 1075
    while (significand & 1L) == 0L do
      significand = significand >> 1
      exponent += 1
    val signed = if d < 0 then -significand else significand
    if exponent >= 0 then new BigDecimal(new BigInteger(bigShl(bigOfLong(signed), exponent)), 0)
    else new BigDecimal(new BigInteger(bigMul(bigOfLong(signed), bigPow(bigOfLong(5L), -exponent))), -exponent)

  @javaDefined
  @jvmClass("java/math/BigDecimal")
  final class BigDecimal(val unscaled: BigInteger, val scaleValue: Int) extends java.lang.Number, java.lang.Comparable[BigDecimal]:
    def this(parsed: BigDecimal) = this(parsed.unscaledValue, parsed.scale)
    def this(text: String) = this(parseDecimal(text))
    def this(text: String, mc: MathContext) = this(parseDecimal(text).round(mc))
    def this(value: Int) = this(BigInteger.valueOf(value.toLong), 0)
    def this(value: Int, mc: MathContext) = this(BigDecimal.valueOf(value.toLong).round(mc))
    def this(value: Long) = this(BigInteger.valueOf(value), 0)
    def this(value: Long, mc: MathContext) = this(BigDecimal.valueOf(value).round(mc))
    def this(value: Double) = this(decimalOfDouble(value))
    def this(value: Double, mc: MathContext) = this(decimalOfDouble(value).round(mc))
    def this(value: BigInteger) = this(value, 0)
    def this(value: BigInteger, mc: MathContext) = this(new BigDecimal(value, 0).round(mc))
    def this(value: BigInteger, scale: Int, mc: MathContext) = this(new BigDecimal(value, scale).round(mc))

    @javaDefined
    def unscaledValue: BigInteger = unscaled
    @javaDefined
    def scale: Int = scaleValue
    @javaDefined
    def precision: Int = digitsOf(unscaled.bits)
    @javaDefined
    def signum: Int = unscaled.signum
    @javaDefined
    def negate: BigDecimal = new BigDecimal(unscaled.negate, scaleValue)
    def negate(mc: MathContext): BigDecimal = negate.round(mc)
    @javaDefined
    def abs: BigDecimal = if signum < 0 then negate else this
    def abs(mc: MathContext): BigDecimal = abs.round(mc)
    @javaDefined
    def plus: BigDecimal = this
    def plus(mc: MathContext): BigDecimal = round(mc)

    private def raised(to: Int): Any = bigMul(unscaled.bits, tenTo(to - scaleValue))
    def add(that: BigDecimal): BigDecimal =
      val s = if scaleValue > that.scaleValue then scaleValue else that.scaleValue
      new BigDecimal(new BigInteger(bigAdd(raised(s), that.raised(s))), s)
    def add(that: BigDecimal, mc: MathContext): BigDecimal = add(that).round(mc)
    def subtract(that: BigDecimal): BigDecimal = add(that.negate)
    def subtract(that: BigDecimal, mc: MathContext): BigDecimal = subtract(that).round(mc)
    def multiply(that: BigDecimal): BigDecimal =
      new BigDecimal(new BigInteger(bigMul(unscaled.bits, that.unscaled.bits)), scaleValue + that.scaleValue)
    def multiply(that: BigDecimal, mc: MathContext): BigDecimal = multiply(that).round(mc)
    def pow(n: Int): BigDecimal =
      if n < 0 || n > 999999999 then throw new java.lang.ArithmeticException("Invalid operation")
      new BigDecimal(unscaled.pow(n), scaleValue * n)
    def pow(n: Int, mc: MathContext): BigDecimal = pow(n).round(mc)

    private def checkDivisor(that: BigDecimal): Unit =
      if that.signum == 0 then
        throw new java.lang.ArithmeticException(if signum == 0 then "Division undefined" else "Division by zero")
    // The quotient at `scale`, rounded as `mode` says.
    def divide(that: BigDecimal, scale: Int, mode: RoundingMode): BigDecimal =
      checkDivisor(that)
      val shift = scale - (scaleValue - that.scaleValue)
      val num = if shift >= 0 then bigMul(unscaled.bits, tenTo(shift)) else unscaled.bits
      val den = if shift >= 0 then that.unscaled.bits else bigMul(that.unscaled.bits, tenTo(-shift))
      val q = if bigSignum(den) < 0 then divideRounded(bigNeg(num), bigNeg(den), mode) else divideRounded(num, den, mode)
      new BigDecimal(new BigInteger(q), scale)
    def divide(that: BigDecimal, mode: RoundingMode): BigDecimal = divide(that, scaleValue, mode)
    def divide(that: BigDecimal, roundingMode: Int): BigDecimal = divide(that, scaleValue, RoundingMode.valueOf(roundingMode))
    def divide(that: BigDecimal, scale: Int, roundingMode: Int): BigDecimal = divide(that, scale, RoundingMode.valueOf(roundingMode))
    // The exact quotient, which exists when the divisor's odd part divides the dividend.
    def divide(that: BigDecimal): BigDecimal =
      checkDivisor(that)
      val preferred = scaleValue - that.scaleValue
      if signum == 0 then return new BigDecimal(BigInteger.ZERO, if preferred < 0 then 0 else preferred)
      val g = bigGcd(unscaled.bits, that.unscaled.bits)
      var rest = bigAbs(bigQuot(that.unscaled.bits, g))
      var twos = 0
      var fives = 0
      while bigIsZero(bigRem(rest, bigOfLong(2L))) do
        rest = bigQuot(rest, bigOfLong(2L))
        twos += 1
      while bigIsZero(bigRem(rest, bigOfLong(5L))) do
        rest = bigQuot(rest, bigOfLong(5L))
        fives += 1
      if !bigSame(rest, bigOfLong(1L)) then throw new java.lang.ArithmeticException("Non-terminating decimal expansion; no exact representable decimal result.")
      val extra = if twos > fives then twos else fives
      divide(that, preferred + extra, RoundingMode.UNNECESSARY).stripZerosTo(preferred)
    // The quotient to the context's precision, then trailing zeros dropped down to the
    // preferred scale.
    def divide(that: BigDecimal, mc: MathContext): BigDecimal =
      if mc.getPrecision == 0 then return divide(that)
      checkDivisor(that)
      val preferred = scaleValue - that.scaleValue
      if signum == 0 then return new BigDecimal(BigInteger.ZERO, if preferred < 0 then 0 else preferred)
      val extra = mc.getPrecision + that.precision - precision + 3
      val shift = if extra < 0 then 0 else extra
      val num = bigMul(unscaled.bits, tenTo(shift))
      var q = bigQuot(num, that.unscaled.bits)
      var s = preferred + shift
      // A non-zero remainder is one sticky digit past the ones kept.
      if !bigIsZero(bigRem(num, that.unscaled.bits)) then
        q = bigAdd(bigMul(q, bigTen), bigOfLong(bigSignum(q).toLong))
        s += 1
      new BigDecimal(new BigInteger(q), s).round(mc).stripZerosTo(preferred)
    private def compareMagnitude(that: BigDecimal): Int = abs.compareTo(that.abs)
    def divideToIntegralValue(that: BigDecimal): BigDecimal =
      val preferred = scaleValue - that.scaleValue
      if compareMagnitude(that) < 0 then return new BigDecimal(BigInteger.ZERO, preferred)
      val q = divide(that, 0, RoundingMode.DOWN).stripZerosTo(preferred)
      if q.scaleValue < preferred then q.setScale(preferred, RoundingMode.UNNECESSARY) else q
    // The integer part of the quotient, which has to fit the context's precision.
    def divideToIntegralValue(that: BigDecimal, mc: MathContext): BigDecimal =
      if mc.getPrecision == 0 || compareMagnitude(that) < 0 then return divideToIntegralValue(that)
      val preferred = scaleValue - that.scaleValue
      var result = divide(that, new MathContext(mc.getPrecision, RoundingMode.DOWN))
      if result.scale < 0 then
        if subtract(result.multiply(that)).compareMagnitude(that) >= 0 then throw new java.lang.ArithmeticException("Division impossible")
      else if result.scale > 0 then result = result.setScale(0, RoundingMode.DOWN)
      val room = mc.getPrecision - result.precision
      if preferred > result.scale && room > 0 then
        result.setScale(result.scale + (if room < preferred - result.scale then room else preferred - result.scale))
      else result.stripZerosTo(preferred)
    def remainder(that: BigDecimal): BigDecimal = subtract(divideToIntegralValue(that).multiply(that))
    def remainder(that: BigDecimal, mc: MathContext): BigDecimal = subtract(divideToIntegralValue(that, mc).multiply(that))
    @jvm("$0 $1 invokevirtual java/math/BigDecimal.divideAndRemainder(Ljava/math/BigDecimal;)[Ljava/math/BigDecimal;")
    def divideAndRemainder(that: BigDecimal): Array[BigDecimal] =
      val q = divideToIntegralValue(that)
      Array(q, subtract(q.multiply(that)))
    @jvm("$0 $1 $2 invokevirtual java/math/BigDecimal.divideAndRemainder(Ljava/math/BigDecimal;Ljava/math/MathContext;)[Ljava/math/BigDecimal;")
    def divideAndRemainder(that: BigDecimal, mc: MathContext): Array[BigDecimal] =
      val q = divideToIntegralValue(that, mc)
      Array(q, subtract(q.multiply(that)))

    def round(mc: MathContext): BigDecimal =
      val wanted = mc.getPrecision
      if wanted == 0 then return this
      var result = this
      while result.precision > wanted do
        val drop = result.precision - wanted
        val q = divideRounded(result.unscaled.bits, tenTo(drop), mc.getRoundingMode)
        result = new BigDecimal(new BigInteger(q), result.scaleValue - drop)
      result
    def setScale(newScale: Int, mode: RoundingMode): BigDecimal =
      if newScale == scaleValue then this
      else if newScale > scaleValue then new BigDecimal(new BigInteger(raised(newScale)), newScale)
      else new BigDecimal(new BigInteger(divideRounded(unscaled.bits, tenTo(scaleValue - newScale), mode)), newScale)
    def setScale(newScale: Int, roundingMode: Int): BigDecimal = setScale(newScale, RoundingMode.valueOf(roundingMode))
    def setScale(newScale: Int): BigDecimal = setScale(newScale, RoundingMode.UNNECESSARY)
    private def stripZerosTo(floor: Int): BigDecimal =
      var u = unscaled.bits
      var s = scaleValue
      while s > floor && !bigIsZero(u) && bigIsZero(bigRem(u, bigTen)) do
        u = bigQuot(u, bigTen)
        s -= 1
      new BigDecimal(new BigInteger(u), s)
    @javaDefined
    def stripTrailingZeros: BigDecimal =
      if signum == 0 then BigDecimal.ZERO else stripZerosTo(java.lang.Integer.MIN_VALUE)
    def scaleByPowerOfTen(n: Int): BigDecimal = new BigDecimal(unscaled, scaleValue - n)
    def movePointLeft(n: Int): BigDecimal =
      val moved = new BigDecimal(unscaled, scaleValue + n)
      if moved.scaleValue < 0 then moved.setScale(0) else moved
    def movePointRight(n: Int): BigDecimal = movePointLeft(-n)
    @javaDefined
    def ulp: BigDecimal = new BigDecimal(BigInteger.ONE, scaleValue)

    def compareTo(that: BigDecimal): Int =
      val s = if scaleValue > that.scaleValue then scaleValue else that.scaleValue
      bigCompare(raised(s), that.raised(s))
    def min(that: BigDecimal): BigDecimal = if compareTo(that) <= 0 then this else that
    def max(that: BigDecimal): BigDecimal = if compareTo(that) >= 0 then this else that
    override def equals(that: Any): Boolean = that match
      case d: BigDecimal => d.scaleValue == scaleValue && bigSame(unscaled.bits, d.unscaled.bits)
      case _ => false
    override def hashCode: Int = 31 * unscaled.hashCode + scaleValue

    @javaDefined
    def toBigInteger: BigInteger =
      if scaleValue <= 0 then new BigInteger(bigMul(unscaled.bits, tenTo(-scaleValue)))
      else new BigInteger(bigQuot(unscaled.bits, tenTo(scaleValue)))
    @javaDefined
    def toBigIntegerExact: BigInteger =
      if scaleValue > 0 && !bigIsZero(bigRem(unscaled.bits, tenTo(scaleValue))) then throw new java.lang.ArithmeticException("Rounding necessary")
      toBigInteger
    @javaDefined
    def intValue: Int = toBigInteger.intValue
    @javaDefined
    def longValue: Long = toBigInteger.longValue
    @javaDefined
    def doubleValue: Double = doubleOfText(toString)
    @javaDefined
    def floatValue: Float = doubleValue.toFloat
    @javaDefined
    def longValueExact: Long = toBigIntegerExact.longValueExact
    @javaDefined
    def intValueExact: Int = toBigIntegerExact.intValueExact
    @javaDefined
    def shortValueExact: Short = toBigIntegerExact.shortValueExact
    @javaDefined
    def byteValueExact: Byte = toBigIntegerExact.byteValueExact

    private def coefficient: String = bigText(bigAbs(unscaled.bits), 10)
    private def signText: String = if signum < 0 then "-" else ""
    private def zeros(n: Int): String =
      var s = ""
      var i = 0
      while i < n do
        s = s + "0"
        i += 1
      s
    // The JDK's form: plain when the scale is zero or the adjusted exponent is at least -6,
    // scientific otherwise.
    override def toString: String =
      val coeff = coefficient
      val adjusted = -scaleValue + (coeff.length - 1)
      if scaleValue == 0 then signText + coeff
      else if scaleValue > 0 && adjusted >= -6 then signText + pointed(coeff)
      else
        val mantissa = if coeff.length > 1 then coeff.substring(0, 1) + "." + coeff.substring(1) else coeff
        val exponent = if adjusted == 0 then "" else if adjusted > 0 then "E+" + adjusted else "E" + adjusted
        signText + mantissa + exponent
    private def pointed(coeff: String): String =
      if coeff.length > scaleValue then coeff.substring(0, coeff.length - scaleValue) + "." + coeff.substring(coeff.length - scaleValue)
      else "0." + zeros(scaleValue - coeff.length) + coeff
    @javaDefined
    def toPlainString: String =
      val coeff = coefficient
      if scaleValue == 0 then signText + coeff
      else if scaleValue < 0 then signText + coeff + zeros(-scaleValue)
      else signText + pointed(coeff)
    @javaDefined
    def toEngineeringString: String = toString

  @javaDefined
  @jvmClass("java/math/BigDecimal")
  object BigDecimal:
    private val zeroValue: BigDecimal = new BigDecimal(BigInteger.ZERO, 0)
    @jvm("getstatic java/math/BigDecimal.ZERO:Ljava/math/BigDecimal;")
    def ZERO: BigDecimal = zeroValue
    private val oneValue: BigDecimal = new BigDecimal(BigInteger.ONE, 0)
    @jvm("getstatic java/math/BigDecimal.ONE:Ljava/math/BigDecimal;")
    def ONE: BigDecimal = oneValue
    private val tenValue: BigDecimal = new BigDecimal(BigInteger.TEN, 0)
    @jvm("getstatic java/math/BigDecimal.TEN:Ljava/math/BigDecimal;")
    def TEN: BigDecimal = tenValue
    @jvm("invokestatic java/math/BigDecimal.valueOf(J)Ljava/math/BigDecimal;")
    def valueOf(l: Long): BigDecimal = new BigDecimal(BigInteger.valueOf(l), 0)
    @jvm("invokestatic java/math/BigDecimal.valueOf(JI)Ljava/math/BigDecimal;")
    def valueOf(l: Long, scale: Int): BigDecimal = new BigDecimal(BigInteger.valueOf(l), scale)
    @jvm("invokestatic java/math/BigDecimal.valueOf(D)Ljava/math/BigDecimal;")
    def valueOf(d: Double): BigDecimal = new BigDecimal(javaDoubleText(d))
    // `Double.toString` as the JDK writes it, which the decimal takes its digits and scale from:
    // the shortest digits that round to the double, plain from 1e-3 to 1e7, `E` notation outside.
    @js("((d) => { if (d !== d) return 'NaN'; if (d === Infinity) return 'Infinity'; if (d === -Infinity) return '-Infinity'; if (d === 0) return 1 / d < 0 ? '-0.0' : '0.0'; const a = d < 0 ? -d : d; const parts = a.toExponential().split('e'); const ds = parts[0].replace('.', ''); const e = +parts[1]; let s; if (a >= 1e-3 && a < 1e7) { if (e < 0) s = '0.' + '0'.repeat(-e - 1) + ds; else if (ds.length > e + 1) s = ds.slice(0, e + 1) + '.' + ds.slice(e + 1); else s = ds + '0'.repeat(e + 1 - ds.length) + '.0'; } else s = ds[0] + '.' + (ds.length > 1 ? ds.slice(1) : '0') + 'E' + e; return d < 0 ? '-' + s : s; })($1)")
    def javaDoubleText(d: Double): String = java.lang.Double.toString(d)
