// `scala.math.BigDecimal` and `BigInt` as scala-library defines them, over `java.math`: the JDK's
// classes on the JVM, `std/javalib/math.scala` elsewhere. Left out when a program defines
// `package scala.math` itself.
package scala.math:

  import java.math.{BigDecimal as BigDec, BigInteger, MathContext}

  final class BigDecimal(val bigDecimal: BigDec, val mc: MathContext) extends java.lang.Number, Ordered[BigDecimal]:
    if bigDecimal eq null then throw new IllegalArgumentException("null value for BigDecimal")
    if mc eq null then throw new IllegalArgumentException("null MathContext for BigDecimal")
    def this(bigDecimal: BigDec) = this(bigDecimal, BigDecimal.defaultMathContext)

    private def wrap(d: BigDec): BigDecimal = new BigDecimal(d, mc)

    def +(that: BigDecimal): BigDecimal = wrap(bigDecimal.add(that.bigDecimal, mc))
    def -(that: BigDecimal): BigDecimal = wrap(bigDecimal.subtract(that.bigDecimal, mc))
    def *(that: BigDecimal): BigDecimal = wrap(bigDecimal.multiply(that.bigDecimal, mc))
    def /(that: BigDecimal): BigDecimal = wrap(bigDecimal.divide(that.bigDecimal, mc))
    def /%(that: BigDecimal): (BigDecimal, BigDecimal) =
      val qr = bigDecimal.divideAndRemainder(that.bigDecimal, mc)
      (wrap(qr(0)), wrap(qr(1)))
    def quot(that: BigDecimal): BigDecimal = wrap(bigDecimal.divideToIntegralValue(that.bigDecimal, mc))
    def remainder(that: BigDecimal): BigDecimal = wrap(bigDecimal.remainder(that.bigDecimal, mc))
    def %(that: BigDecimal): BigDecimal = remainder(that)
    def pow(n: Int): BigDecimal = wrap(bigDecimal.pow(n, mc))
    def unary_- : BigDecimal = wrap(bigDecimal.negate(mc))
    def abs: BigDecimal = if signum < 0 then -this else this
    def signum: Int = bigDecimal.signum()
    def sign: BigDecimal = BigDecimal(signum)
    def min(that: BigDecimal): BigDecimal = if compare(that) <= 0 then this else that
    def max(that: BigDecimal): BigDecimal = if compare(that) >= 0 then this else that

    def compare(that: BigDecimal): Int = bigDecimal.compareTo(that.bigDecimal)

    def precision: Int = bigDecimal.precision()
    def scale: Int = bigDecimal.scale()
    def ulp: BigDecimal = wrap(bigDecimal.ulp())
    def round(mc: MathContext): BigDecimal =
      val r = bigDecimal.round(mc)
      if r eq bigDecimal then this else wrap(r)
    def rounded: BigDecimal = round(mc)
    def apply(mc: MathContext): BigDecimal = new BigDecimal(bigDecimal.round(mc), mc)
    def setScale(scale: Int): BigDecimal =
      if this.scale == scale then this else wrap(bigDecimal.setScale(scale))
    def setScale(scale: Int, mode: BigDecimal.RoundingMode.Value): BigDecimal =
      if this.scale == scale then this else wrap(bigDecimal.setScale(scale, java.math.RoundingMode.valueOf(mode.id)))
    def underlying: BigDec = bigDecimal

    def isWhole: Boolean = scale <= 0 || bigDecimal.stripTrailingZeros().scale() <= 0
    private def exactly(f: => Any): Boolean =
      try
        f
        true
      catch case _: ArithmeticException => false
    def isValidByte: Boolean = exactly(bigDecimal.byteValueExact())
    def isValidShort: Boolean = exactly(bigDecimal.shortValueExact())
    def isValidChar: Boolean = isValidInt && toIntExact >= 0 && toIntExact <= 65535
    def isValidInt: Boolean = exactly(bigDecimal.intValueExact())
    def isValidLong: Boolean = exactly(bigDecimal.longValueExact())
    def isDecimalDouble: Boolean =
      val d = toDouble
      !d.isInfinite && equals(BigDecimal.decimal(d, mc))
    def isExactDouble: Boolean =
      val d = toDouble
      !d.isInfinite && equals(BigDecimal.exact(d))

    def toByte: Byte = intValue.toByte
    def toShort: Short = intValue.toShort
    def toChar: Char = intValue.toChar
    def toInt: Int = intValue
    def toLong: Long = longValue
    def toFloat: Float = floatValue
    def toDouble: Double = doubleValue
    def intValue: Int = bigDecimal.intValue()
    def longValue: Long = bigDecimal.longValue()
    def floatValue: Float = bigDecimal.floatValue()
    def doubleValue: Double = bigDecimal.doubleValue()
    def toIntExact: Int = bigDecimal.intValueExact()
    def toLongExact: Long = bigDecimal.longValueExact()
    def toBigInt: BigInt = new BigInt(bigDecimal.toBigInteger())
    def toBigIntExact: Option[BigInt] =
      if !isWhole then None
      else
        try Some(new BigInt(bigDecimal.toBigIntegerExact()))
        catch case _: ArithmeticException => None

    def equals(that: BigDecimal): Boolean = compare(that) == 0
    override def equals(that: Any): Boolean = that match
      case that: BigDecimal => compare(that) == 0
      case that: BigInt =>
        that.bitLength > (precision - scale - 2) * 3.3219280948873626 && toBigIntExact.exists(that.equals(_))
      case that: Double =>
        !that.isInfinite && {
          val d = toDouble
          !d.isInfinite && d == that && equals(BigDecimal.decimal(d, mc))
        }
      case that: Float =>
        !that.isInfinite && {
          val f = toFloat
          !f.isInfinite && f == that && equals(BigDecimal.decimal(f.toDouble, mc))
        }
      case that: Long => isValidLong && toLong == that
      case that: Int => isValidInt && toInt == that
      case that: Short => isValidShort && toShort == that
      case that: Byte => isValidByte && toByte == that
      case that: Char => isValidChar && toInt == that.toInt
      case _ => false

    // A whole number hashes as the `BigInt` it equals, a number a double holds as that double.
    override def hashCode: Int =
      if isWhole && precision - scale < 4934 then toBigInt.hashCode
      else if isDecimalDouble then doubleValue.##
      else
        val temp = bigDecimal.stripTrailingZeros()
        BigDecimal.mixLast(temp.scaleByPowerOfTen(temp.scale()).toBigInteger().hashCode, temp.scale())

    override def toString: String = bigDecimal.toString

  object BigDecimal:
    val defaultMathContext: MathContext = MathContext.DECIMAL128

    // scala-library's `Enumeration` of the rounding modes, with the ids of `java.math.RoundingMode`.
    object RoundingMode:
      final class Value(val id: Int, name: String) extends Ordered[Value]:
        def compare(that: Value): Int = id - that.id
        override def toString: String = name
      type RoundingMode = Value
      val UP: Value = new Value(0, "UP")
      val DOWN: Value = new Value(1, "DOWN")
      val CEILING: Value = new Value(2, "CEILING")
      val FLOOR: Value = new Value(3, "FLOOR")
      val HALF_UP: Value = new Value(4, "HALF_UP")
      val HALF_DOWN: Value = new Value(5, "HALF_DOWN")
      val HALF_EVEN: Value = new Value(6, "HALF_EVEN")
      val UNNECESSARY: Value = new Value(7, "UNNECESSARY")
      def values: List[Value] = List(UP, DOWN, CEILING, FLOOR, HALF_UP, HALF_DOWN, HALF_EVEN, UNNECESSARY)
      def maxId: Int = 8
      def apply(id: Int): Value = values(id)
      def withName(name: String): Value = values.find(_.toString == name) match
        case Some(v) => v
        case None => throw new NoSuchElementException("No value found for '" + name + "'")

    def mixLast(hash: Int, data: Int): Int =
      var k = data * -862048943
      k = (k << 15) | (k >>> 17)
      k = k * 461845907
      hash ^ k

    def decimal(d: Double, mc: MathContext): BigDecimal = new BigDecimal(BigDec.valueOf(d).round(mc), mc)
    def decimal(d: Double): BigDecimal = decimal(d, defaultMathContext)
    def decimal(l: Long, mc: MathContext): BigDecimal = apply(l, mc)
    def decimal(l: Long): BigDecimal = apply(l)
    def decimal(bd: BigDec, mc: MathContext): BigDecimal = new BigDecimal(bd.round(mc), mc)
    def binary(d: Double, mc: MathContext): BigDecimal = new BigDecimal(new BigDec(d, mc), mc)
    def binary(d: Double): BigDecimal = binary(d, defaultMathContext)

    // An exact value keeps all its digits: the context widens to its precision.
    def exact(repr: BigDec): BigDecimal =
      val context =
        if repr.precision() <= defaultMathContext.getPrecision() then defaultMathContext
        else new MathContext(repr.precision(), java.math.RoundingMode.HALF_EVEN)
      new BigDecimal(repr, context)
    def exact(d: Double): BigDecimal = exact(new BigDec(d))
    def exact(bi: BigInt): BigDecimal = exact(new BigDec(bi.bigInteger))
    def exact(l: Long): BigDecimal = apply(l)
    def exact(s: String): BigDecimal = exact(new BigDec(s))

    def valueOf(d: Double): BigDecimal = apply(BigDec.valueOf(d))
    def valueOf(x: Long): BigDecimal = apply(x)

    def apply(i: Int): BigDecimal = apply(i.toLong)
    def apply(i: Int, mc: MathContext): BigDecimal = apply(i.toLong, mc)
    def apply(l: Long): BigDecimal = new BigDecimal(BigDec.valueOf(l), defaultMathContext)
    def apply(l: Long, mc: MathContext): BigDecimal = new BigDecimal(new BigDec(l, mc), mc)
    def apply(unscaledVal: Long, scale: Int): BigDecimal = apply(BigInt(unscaledVal), scale)
    def apply(unscaledVal: Long, scale: Int, mc: MathContext): BigDecimal = apply(BigInt(unscaledVal), scale, mc)
    def apply(d: Double): BigDecimal = decimal(d)
    def apply(d: Double, mc: MathContext): BigDecimal = decimal(d, mc)
    def apply(x: String): BigDecimal = exact(x)
    def apply(x: Array[Char]): BigDecimal = exact(new BigDec(x))
    def apply(x: Array[Char], mc: MathContext): BigDecimal = new BigDecimal(new BigDec(x, mc), mc)
    def apply(x: String, mc: MathContext): BigDecimal = new BigDecimal(new BigDec(x, mc), mc)
    def apply(x: BigInt): BigDecimal = exact(x)
    def apply(x: BigInt, mc: MathContext): BigDecimal = new BigDecimal(new BigDec(x.bigInteger, mc), mc)
    def apply(unscaledVal: BigInt, scale: Int): BigDecimal = exact(new BigDec(unscaledVal.bigInteger, scale))
    def apply(unscaledVal: BigInt, scale: Int, mc: MathContext): BigDecimal =
      new BigDecimal(new BigDec(unscaledVal.bigInteger, scale, mc), mc)
    def apply(bd: BigDec): BigDecimal = new BigDecimal(bd, defaultMathContext)
    def apply(bd: BigDec, mc: MathContext): BigDecimal = new BigDecimal(bd, mc)

    given ordering: Ordering[BigDecimal] with
      def compare(a: BigDecimal, b: BigDecimal): Int = a.compare(b)
    given BigDecimalIsFractional: Fractional[BigDecimal] with
      def zero: BigDecimal = BigDecimal(0)
      def one: BigDecimal = BigDecimal(1)
      def plus(a: BigDecimal, b: BigDecimal): BigDecimal = a + b
      def minus(a: BigDecimal, b: BigDecimal): BigDecimal = a - b
      def times(a: BigDecimal, b: BigDecimal): BigDecimal = a * b
      def div(a: BigDecimal, b: BigDecimal): BigDecimal = a / b
      def fromInt(x: Int): BigDecimal = BigDecimal(x)
      def toInt(x: BigDecimal): Int = x.intValue
      def toLong(x: BigDecimal): Long = x.longValue
      def toDouble(x: BigDecimal): Double = x.doubleValue
      def compare(a: BigDecimal, b: BigDecimal): Int = a.compare(b)

    implicit def int2bigDecimal(i: Int): BigDecimal = apply(i)
    implicit def long2bigDecimal(l: Long): BigDecimal = apply(l)
    implicit def double2bigDecimal(d: Double): BigDecimal = decimal(d)
    implicit def javaBigDecimal2bigDecimal(x: BigDec): BigDecimal = if x == null then null else apply(x)

  final class BigInt(val bigInteger: BigInteger) extends java.lang.Number, Ordered[BigInt]:
    private def wrap(b: BigInteger): BigInt = new BigInt(b)
    private def bothLong(that: BigInt): Boolean = isValidLong && that.isValidLong

    def +(that: BigInt): BigInt = wrap(bigInteger.add(that.bigInteger))
    def -(that: BigInt): BigInt = wrap(bigInteger.subtract(that.bigInteger))
    def *(that: BigInt): BigInt = wrap(bigInteger.multiply(that.bigInteger))
    // Two values that fit a `Long` divide as `Long`s, which is where the message comes from.
    def /(that: BigInt): BigInt =
      if that.signum == 0 && bothLong(that) then throw new ArithmeticException("/ by zero")
      wrap(bigInteger.divide(that.bigInteger))
    def %(that: BigInt): BigInt =
      if that.signum == 0 && bothLong(that) then throw new ArithmeticException("/ by zero")
      wrap(bigInteger.remainder(that.bigInteger))
    def /%(that: BigInt): (BigInt, BigInt) = (this / that, this % that)
    def <<(n: Int): BigInt = wrap(bigInteger.shiftLeft(n))
    def >>(n: Int): BigInt = wrap(bigInteger.shiftRight(n))
    def &(that: BigInt): BigInt = wrap(bigInteger.and(that.bigInteger))
    def |(that: BigInt): BigInt = wrap(bigInteger.or(that.bigInteger))
    def ^(that: BigInt): BigInt = wrap(bigInteger.xor(that.bigInteger))
    def &~(that: BigInt): BigInt = wrap(bigInteger.andNot(that.bigInteger))
    def gcd(that: BigInt): BigInt = wrap(bigInteger.gcd(that.bigInteger))
    def mod(that: BigInt): BigInt = wrap(bigInteger.mod(that.bigInteger))
    def modPow(exp: BigInt, m: BigInt): BigInt = wrap(bigInteger.modPow(exp.bigInteger, m.bigInteger))
    def modInverse(m: BigInt): BigInt = wrap(bigInteger.modInverse(m.bigInteger))
    def pow(exp: Int): BigInt = wrap(bigInteger.pow(exp))
    def unary_- : BigInt = wrap(bigInteger.negate())
    def unary_~ : BigInt = wrap(bigInteger.not())
    def abs: BigInt = if signum < 0 then -this else this
    def signum: Int = bigInteger.signum()
    def sign: BigInt = BigInt(signum)
    def min(that: BigInt): BigInt = if compare(that) <= 0 then this else that
    def max(that: BigInt): BigInt = if compare(that) >= 0 then this else that

    def compare(that: BigInt): Int = bigInteger.compareTo(that.bigInteger)

    def testBit(n: Int): Boolean = bigInteger.testBit(n)
    def setBit(n: Int): BigInt = wrap(bigInteger.setBit(n))
    def clearBit(n: Int): BigInt = wrap(bigInteger.clearBit(n))
    def flipBit(n: Int): BigInt = wrap(bigInteger.flipBit(n))
    def lowestSetBit: Int = bigInteger.getLowestSetBit()
    def bitLength: Int = bigInteger.bitLength()
    def bitCount: Int = bigInteger.bitCount()
    def isProbablePrime(certainty: Int): Boolean = bigInteger.isProbablePrime(certainty)
    // The two's complement, most significant byte first.
    def toByteArray: Array[Byte] =
      val n = bitLength / 8 + 1
      Array.tabulate(n)(i => bigInteger.shiftRight(8 * (n - 1 - i)).intValue.toByte)
    def toString(radix: Int): String = bigInteger.toString(radix)
    def underlying: BigInteger = bigInteger
    def isWhole: Boolean = true

    def isValidByte: Boolean = bitLength <= 7
    def isValidShort: Boolean = bitLength <= 15
    def isValidChar: Boolean = signum >= 0 && bitLength <= 16
    def isValidInt: Boolean = bitLength <= 31
    def isValidLong: Boolean = bitLength <= 63
    def isValidFloat: Boolean =
      val bits = bitLength
      bits <= 24 || { val lowest = lowestSetBit; bits <= 128 && lowest >= bits - 24 && lowest < 128 }
    def isValidDouble: Boolean =
      val bits = bitLength
      bits <= 53 || { val lowest = lowestSetBit; bits <= 1024 && lowest >= bits - 53 && lowest < 1024 }

    def toByte: Byte = intValue.toByte
    def toShort: Short = intValue.toShort
    def toChar: Char = intValue.toChar
    def toInt: Int = intValue
    def toLong: Long = longValue
    def toFloat: Float = floatValue
    def toDouble: Double = doubleValue
    def intValue: Int = bigInteger.intValue()
    def longValue: Long = bigInteger.longValue()
    def floatValue: Float = bigInteger.floatValue()
    def doubleValue: Double = bigInteger.doubleValue()

    def equals(that: BigInt): Boolean = compare(that) == 0
    override def equals(that: Any): Boolean = that match
      case that: BigInt => compare(that) == 0
      case that: BigDecimal => that.equals(this)
      case that: Double => isValidDouble && toDouble == that
      case that: Float => isValidFloat && toFloat == that
      case that: Long => isValidLong && toLong == that
      case that: Int => isValidInt && toInt == that
      case that: Short => isValidShort && toShort == that
      case that: Byte => isValidByte && toByte == that
      case that: Char => isValidChar && toInt == that.toInt
      case _ => false
    // A value that fits a `Long` hashes as that `Long` does.
    override def hashCode: Int =
      if isValidLong then
        val l = longValue
        if l.toInt.toLong == l then l.toInt else (l ^ (l >>> 32)).toInt
      else bigInteger.hashCode
    override def toString: String = bigInteger.toString

  object BigInt:
    def apply(i: Int): BigInt = new BigInt(BigInteger.valueOf(i.toLong))
    def apply(l: Long): BigInt = new BigInt(BigInteger.valueOf(l))
    def apply(x: Array[Byte]): BigInt =
      if x.length == 0 then throw new NumberFormatException("Zero length BigInteger")
      val unsigned = BigInt(1, x)
      if x(0) < 0 then unsigned - (BigInt(1) << (8 * x.length)) else unsigned
    def apply(signum: Int, magnitude: Array[Byte]): BigInt =
      if signum < -1 || signum > 1 then throw new NumberFormatException("Invalid signum value")
      var acc = BigInteger.ZERO
      for b <- magnitude do acc = acc.shiftLeft(8).or(BigInteger.valueOf((b & 255).toLong))
      if signum == 0 && acc.signum() != 0 then throw new NumberFormatException("signum-magnitude mismatch")
      new BigInt(if signum < 0 then acc.negate() else acc)
    def apply(x: String): BigInt = new BigInt(new BigInteger(x))
    def apply(x: String, radix: Int): BigInt = new BigInt(new BigInteger(x, radix))
    def apply(x: BigInteger): BigInt = new BigInt(x)
    // scala-library's: the JDK's random constructors and `probablePrime` over the wrapped generator.
    def apply(numbits: Int, rnd: scala.util.Random): BigInt = new BigInt(new BigInteger(numbits, rnd.self))
    def apply(bitlength: Int, certainty: Int, rnd: scala.util.Random): BigInt = new BigInt(new BigInteger(bitlength, certainty, rnd.self))
    def probablePrime(bitLength: Int, rnd: scala.util.Random): BigInt = new BigInt(BigInteger.probablePrime(bitLength, rnd.self))
    val MinLong: BigInt = BigInt(Long.MinValue)
    val MaxLong: BigInt = BigInt(Long.MaxValue)
    given ordering: Ordering[BigInt] with
      def compare(a: BigInt, b: BigInt): Int = a.compare(b)
    given BigIntIsIntegral: Integral[BigInt] with
      def zero: BigInt = BigInt(0)
      def one: BigInt = BigInt(1)
      def plus(a: BigInt, b: BigInt): BigInt = a + b
      def minus(a: BigInt, b: BigInt): BigInt = a - b
      def times(a: BigInt, b: BigInt): BigInt = a * b
      def quot(a: BigInt, b: BigInt): BigInt = a / b
      def rem(a: BigInt, b: BigInt): BigInt = a % b
      def fromInt(x: Int): BigInt = BigInt(x)
      def toInt(x: BigInt): Int = x.intValue
      def toLong(x: BigInt): Long = x.longValue
      def toDouble(x: BigInt): Double = x.doubleValue
      def compare(a: BigInt, b: BigInt): Int = a.compare(b)

    implicit def int2bigInt(i: Int): BigInt = apply(i)
    implicit def long2bigInt(l: Long): BigInt = apply(l)
    implicit def javaBigInteger2bigInt(x: BigInteger): BigInt = if x == null then null else apply(x)

package scala:
  export scala.math.{BigDecimal, BigInt}
