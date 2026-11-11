// A JDK class's static final field without a constant value is a stable value to dotty's classfile parser, so it is
// a stable identifier in a pattern; one with a constant value is the constant.
import java.util.{Locale, TimeZone}
import java.nio.charset.{Charset, StandardCharsets}
import java.math.{BigDecimal, BigInteger, MathContext}
import java.nio.ByteOrder

def locale(l: Locale): String = l match
  case Locale.US => "us"
  case Locale.FRANCE => "france"
  case _ => "other"

def charset(c: Charset): String = c match
  case StandardCharsets.UTF_8 => "utf-8"
  case _ => "other"

def integer(i: BigInteger): String = i match
  case BigInteger.ZERO => "zero"
  case BigInteger.ONE => "one"
  case BigInteger.TWO => "two"
  case BigInteger.TEN => "ten"
  case _ => "other"

def decimal(d: BigDecimal): String = d match
  case BigDecimal.ZERO => "zero"
  case BigDecimal.ONE => "one"
  case BigDecimal.TEN => "ten"
  case _ => "other"

def order(o: ByteOrder): String = o match
  case ByteOrder.BIG_ENDIAN => "big"
  case ByteOrder.LITTLE_ENDIAN => "little"

def context(m: MathContext): String = m match
  case MathContext.DECIMAL32 => "32"
  case MathContext.DECIMAL64 => "64"
  case MathContext.DECIMAL128 => "128"
  case MathContext.UNLIMITED => "unlimited"
  case _ => "other"

def style(s: Int): String = s match
  case TimeZone.SHORT => "short"
  case TimeZone.LONG => "long"
  case _ => "other"

def primitive(c: Class[?]): String = c match
  case Integer.TYPE => "int"
  case java.lang.Long.TYPE => "long"
  case _ => "other"

@main def run(): Unit =
  println(locale(Locale.US) + " " + locale(Locale.FRANCE) + " " + locale(Locale.JAPAN))
  println(charset(StandardCharsets.UTF_8) + " " + charset(StandardCharsets.US_ASCII))
  println(integer(BigInteger.ZERO) + " " + integer(BigInteger.ONE) + " " + integer(BigInteger.TWO) + " " + integer(BigInteger.TEN) + " " + integer(BigInteger.valueOf(7)))
  println(decimal(BigDecimal.ZERO) + " " + decimal(BigDecimal.ONE) + " " + decimal(BigDecimal.TEN))
  println(order(ByteOrder.BIG_ENDIAN) + " " + order(ByteOrder.LITTLE_ENDIAN))
  println(context(MathContext.DECIMAL32) + " " + context(MathContext.DECIMAL64) + " " + context(MathContext.DECIMAL128) + " " + context(MathContext.UNLIMITED))
  println(style(TimeZone.SHORT) + " " + style(TimeZone.LONG) + " " + style(5))
  println(primitive(classOf[Int]) + " " + primitive(classOf[Long]) + " " + primitive(classOf[String]))
  val us: Locale.US.type = Locale.US
  val utf8: StandardCharsets.UTF_8.type = StandardCharsets.UTF_8
  val short: 0 = TimeZone.SHORT
  println(us.toString + " " + utf8 + " " + short)
