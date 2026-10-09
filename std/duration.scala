package scala.concurrent.duration

sealed abstract class Duration:
  def length: Long
  def unit: TimeUnit
  def toNanos: Long
  def toMicros: Long
  def toMillis: Long
  def toSeconds: Long
  def toMinutes: Long
  def toHours: Long
  def toDays: Long
  def toUnit(unit: TimeUnit): Double
  def +(other: Duration): Duration
  def -(other: Duration): Duration
  def *(factor: Double): Duration
  def /(divisor: Double): Duration
  def /(divisor: Duration): Double
  def unary_- : Duration
  def isFinite: Boolean
  def toCoarsest: Duration
  def compare(other: Duration): Int
  def <(other: Duration): Boolean = compare(other) < 0
  def <=(other: Duration): Boolean = compare(other) <= 0
  def >(other: Duration): Boolean = compare(other) > 0
  def >=(other: Duration): Boolean = compare(other) >= 0
  def min(other: Duration): Duration = if this < other then this else other
  def max(other: Duration): Duration = if this > other then this else other
  def plus(other: Duration): Duration = this + other
  def minus(other: Duration): Duration = this - other
  def mul(factor: Double): Duration = this * factor
  def div(divisor: Double): Duration = this / divisor
  def div(other: Duration): Double = this / other
  def neg(): Duration = -this
  def gt(other: Duration): Boolean = this > other
  def gteq(other: Duration): Boolean = this >= other
  def lt(other: Duration): Boolean = this < other
  def lteq(other: Duration): Boolean = this <= other

final class FiniteDuration(val length: Long, val unitNanos: Long) extends Duration:
  def this(length: Long, unit: TimeUnit) = this(length, unit.toNanos(1L))
  def unit: TimeUnit =
    if unitNanos == 1L then NANOSECONDS
    else if unitNanos == 1000L then MICROSECONDS
    else if unitNanos == 1000000L then MILLISECONDS
    else if unitNanos == 1000000000L then SECONDS
    else if unitNanos == 60000000000L then MINUTES
    else if unitNanos == 3600000000000L then HOURS
    else DAYS
  def toUnit(unit: TimeUnit): Double = toNanos.toDouble / unit.toNanos(1L).toDouble
  def toNanos: Long = length * unitNanos
  def toMicros: Long = toNanos / 1000L
  def toMillis: Long = toNanos / 1000000L
  def toSeconds: Long = toNanos / 1000000000L
  def toMinutes: Long = toNanos / 60000000000L
  def toHours: Long = toNanos / 3600000000000L
  def toDays: Long = toNanos / 86400000000000L
  private def inUnit(nanos: Long, unit: Long): FiniteDuration = new FiniteDuration(nanos / unit, unit)
  def +(other: FiniteDuration): FiniteDuration = inUnit(toNanos + other.toNanos, unitNanos.min(other.unitNanos))
  def -(other: FiniteDuration): FiniteDuration = inUnit(toNanos - other.toNanos, unitNanos.min(other.unitNanos))
  def +(other: Duration): Duration = other match
    case f: FiniteDuration => this + f
    case _ => other
  def -(other: Duration): Duration = other match
    case f: FiniteDuration => this - f
    case _ => -other
  def *(factor: Long): FiniteDuration = new FiniteDuration(length * factor, unitNanos)
  def /(divisor: Long): FiniteDuration = Duration.fromNanos(toNanos / divisor)
  def *(factor: Double): Duration =
    if !factor.isInfinite then Duration.fromNanos(toNanos.toDouble * factor)
    else if factor.isNaN then Duration.Undefined
    else if (factor > 0d) != (toNanos < 0L) then Duration.Inf
    else Duration.MinusInf
  def /(divisor: Double): Duration =
    if !divisor.isInfinite then Duration.fromNanos(toNanos.toDouble / divisor)
    else if divisor.isNaN then Duration.Undefined
    else Duration.Zero
  def /(divisor: Duration): Double =
    if divisor.isFinite then toNanos.toDouble / divisor.toNanos.toDouble
    else if divisor eq Duration.Undefined then Double.NaN
    else 0d
  def plus(other: FiniteDuration): FiniteDuration = this + other
  def minus(other: FiniteDuration): FiniteDuration = this - other
  def mul(factor: Long): FiniteDuration = this * factor
  def div(divisor: Long): FiniteDuration = this / divisor
  def unary_- : FiniteDuration = new FiniteDuration(-length, unitNanos)
  def compare(other: Duration): Int = other match
    case f: FiniteDuration => comparePrimitives(toNanos, f.toNanos)
    case _ => -other.compare(this)
  def max(other: FiniteDuration): FiniteDuration = if this >= other then this else other
  def min(other: FiniteDuration): FiniteDuration = if this <= other then this else other
  def isFinite: Boolean = true
  def toCoarsest: FiniteDuration = Duration.fromNanos(toNanos)
  def equals(that: Any): Boolean = that match
    case d: FiniteDuration => toNanos == d.toNanos
    case _ => false
  override def hashCode: Int = toNanos.hashCode
  override def toString: String =
    val name =
      if unitNanos == 1L then "nanosecond"
      else if unitNanos == 1000L then "microsecond"
      else if unitNanos == 1000000L then "millisecond"
      else if unitNanos == 1000000000L then "second"
      else if unitNanos == 60000000000L then "minute"
      else if unitNanos == 3600000000000L then "hour"
      else "day"
    length.toString + " " + name + (if length == 1L then "" else "s")

object Duration:
  val Zero: FiniteDuration = new FiniteDuration(0L, 86400000000000L)
  def apply(length: Long, unit: TimeUnit): FiniteDuration = new FiniteDuration(length, unit.toNanos(1L))
  def apply(length: Double, unit: TimeUnit): Duration = fromNanos(unit.toNanos(1L).toDouble * length)
  def apply(length: Long, unit: String): FiniteDuration = apply(length, timeUnit.getOrElse(unit, throw new NoSuchElementException("key not found: " + unit)))
  /** scala-library's parse of `"5 seconds"`, `"1.5h"`, `"Inf"` and `"-Inf"`. */
  def apply(s: String): Duration =
    val text = s.filterNot(_.isWhitespace)
    text match
      case "Inf" | "PlusInf" | "+Inf" => Inf
      case "MinusInf" | "-Inf" => MinusInf
      case _ =>
        val unitName = text.reverse.takeWhile(_.isLetter).reverse
        timeUnit.get(unitName) match
          case Some(unit) =>
            val value = text.dropRight(unitName.length)
            val d = java.lang.Double.parseDouble(value)
            if d >= -9007199254740992d && d <= 9007199254740992d then apply(d, unit)
            else apply(java.lang.Long.parseLong(value), unit)
          case None => throw new NumberFormatException("format error " + s)
  private val timeUnit: Map[String, TimeUnit] =
    List(
      DAYS -> "d day",
      HOURS -> "h hr hour",
      MINUTES -> "m min minute",
      SECONDS -> "s sec second",
      MILLISECONDS -> "ms milli millisecond",
      MICROSECONDS -> "µs micro microsecond",
      NANOSECONDS -> "ns nano nanosecond",
    ).flatMap { (unit, labels) =>
      val first :: rest = labels.split(" ").toList: @unchecked
      (first :: rest.flatMap(l => List(l, l + "s"))).map(_ -> unit)
    }.toMap
  def create(length: Long, unit: TimeUnit): FiniteDuration = apply(length, unit)
  def create(length: Double, unit: TimeUnit): Duration = apply(length, unit)
  def unapply(d: Duration): Option[(Long, TimeUnit)] = if d.isFinite then Some((d.length, d.unit)) else None
  // The coarsest unit that holds the value exactly.
  def fromNanos(nanos: Long): FiniteDuration =
    val units = List(86400000000000L, 3600000000000L, 60000000000L, 1000000000L, 1000000L, 1000L, 1L)
    val unit = units.find(u => nanos % u == 0L).getOrElse(1L)
    new FiniteDuration(nanos / unit, unit)
  def fromNanos(nanos: Double): Duration =
    if nanos.isInfinite then (if nanos > 0d then Inf else MinusInf)
    else if nanos.isNaN then Undefined
    else if nanos > 9.223372036854775807e18 || nanos < -9.223372036854775808e18 then
      throw new IllegalArgumentException("trying to construct too large duration with " + nanos + "ns")
    else fromNanos(nanos.round)

  sealed abstract class Infinite extends Duration:
    def +(other: Duration): Duration =
      if (other eq Undefined) || (other.isInstanceOf[Infinite] && (other ne this)) then Undefined else this
    def -(other: Duration): Duration =
      if (other eq Undefined) || (other eq this) then Undefined else this
    def *(factor: Double): Duration =
      if factor == 0d || factor.isNaN then Undefined else if factor < 0d then -this else this
    def /(divisor: Double): Duration =
      if divisor.isNaN || divisor.isInfinite then Undefined else if divisor < 0d then -this else this
    def /(divisor: Duration): Double = divisor match
      case _: Infinite => Double.NaN
      case _ => if (this > Zero) != (divisor >= Zero) then Double.NegativeInfinity else Double.PositiveInfinity
    final def isFinite: Boolean = false
    private def fail(what: String): Nothing = throw new IllegalArgumentException(what + " not allowed on infinite Durations")
    final def length: Long = fail("length")
    final def unit: TimeUnit = fail("unit")
    final def toNanos: Long = fail("toNanos")
    final def toMicros: Long = fail("toMicros")
    final def toMillis: Long = fail("toMillis")
    final def toSeconds: Long = fail("toSeconds")
    final def toMinutes: Long = fail("toMinutes")
    final def toHours: Long = fail("toHours")
    final def toDays: Long = fail("toDays")
    final def toCoarsest: Duration = this

  private final class Unbounded(positive: Boolean) extends Infinite:
    def toUnit(unit: TimeUnit): Double = if positive then Double.PositiveInfinity else Double.NegativeInfinity
    def unary_- : Duration = if positive then MinusInf else Inf
    def compare(other: Duration): Int =
      if other eq this then 0 else if positive then (if other eq Undefined then -1 else 1) else -1
    override def toString: String = if positive then "Duration.Inf" else "Duration.MinusInf"

  private final class Unknown extends Infinite:
    override def +(other: Duration): Duration = this
    override def -(other: Duration): Duration = this
    override def *(factor: Double): Duration = this
    override def /(divisor: Double): Duration = this
    override def /(divisor: Duration): Double = Double.NaN
    def toUnit(unit: TimeUnit): Double = Double.NaN
    def unary_- : Duration = this
    def compare(other: Duration): Int = if other eq this then 0 else 1
    override def equals(other: Any): Boolean = false
    override def toString: String = "Duration.Undefined"

  val Inf: Infinite = new Unbounded(true)
  val MinusInf: Infinite = new Unbounded(false)
  val Undefined: Infinite = new Unknown

  given DurationIsOrdered: Ordering[Duration] = Ordering.fromCompare((a, b) => a.compare(b))

object FiniteDuration:
  given FiniteDurationIsOrdered: Ordering[FiniteDuration] = Ordering.fromCompare((a, b) => a.compare(b))
  def apply(length: Long, unit: TimeUnit): FiniteDuration = new FiniteDuration(length, unit.toNanos(1L))

/** A point in time on the clock of `System.nanoTime`. */
final case class Deadline private (time: FiniteDuration) extends Ordered[Deadline]:
  def +(other: FiniteDuration): Deadline = Deadline(time + other)
  def -(other: FiniteDuration): Deadline = Deadline(time - other)
  def -(other: Deadline): FiniteDuration = time - other.time
  def timeLeft: FiniteDuration = this - Deadline.now
  def hasTimeLeft(): Boolean = !isOverdue()
  def isOverdue(): Boolean = time.toNanos - System.nanoTime() < 0
  def compare(other: Deadline): Int = time.compare(other.time)

object Deadline:
  def now: Deadline = Deadline(FiniteDuration(System.nanoTime(), NANOSECONDS))
  given DeadlineIsOrdered: Ordering[Deadline] = Ordering.fromCompare((a, b) => a.compare(b))

// scala-library's package object, whose names a library's bodies bind to.
type TimeUnit = java.util.concurrent.TimeUnit
val NANOSECONDS: TimeUnit = java.util.concurrent.TimeUnit.NANOSECONDS
val MICROSECONDS: TimeUnit = java.util.concurrent.TimeUnit.MICROSECONDS
val MILLISECONDS: TimeUnit = java.util.concurrent.TimeUnit.MILLISECONDS
val SECONDS: TimeUnit = java.util.concurrent.TimeUnit.SECONDS
val MINUTES: TimeUnit = java.util.concurrent.TimeUnit.MINUTES
val HOURS: TimeUnit = java.util.concurrent.TimeUnit.HOURS
val DAYS: TimeUnit = java.util.concurrent.TimeUnit.DAYS

trait DurationConversions:
  protected def durationIn(unit: TimeUnit): FiniteDuration
  def nanoseconds: FiniteDuration = durationIn(NANOSECONDS)
  def nanos: FiniteDuration = nanoseconds
  def nanosecond: FiniteDuration = nanoseconds
  def nano: FiniteDuration = nanoseconds
  def microseconds: FiniteDuration = durationIn(MICROSECONDS)
  def micros: FiniteDuration = microseconds
  def microsecond: FiniteDuration = microseconds
  def micro: FiniteDuration = microseconds
  def milliseconds: FiniteDuration = durationIn(MILLISECONDS)
  def millis: FiniteDuration = milliseconds
  def millisecond: FiniteDuration = milliseconds
  def milli: FiniteDuration = milliseconds
  def seconds: FiniteDuration = durationIn(SECONDS)
  def second: FiniteDuration = seconds
  def minutes: FiniteDuration = durationIn(MINUTES)
  def minute: FiniteDuration = minutes
  def hours: FiniteDuration = durationIn(HOURS)
  def hour: FiniteDuration = hours
  def days: FiniteDuration = durationIn(DAYS)
  def day: FiniteDuration = days

implicit final class DurationInt(n: Int) extends DurationConversions:
  protected def durationIn(unit: TimeUnit): FiniteDuration = new FiniteDuration(n.toLong, unit.toNanos(1L))

implicit final class DurationLong(n: Long) extends DurationConversions:
  protected def durationIn(unit: TimeUnit): FiniteDuration = new FiniteDuration(n, unit.toNanos(1L))

implicit final class DurationDouble(d: Double) extends DurationConversions:
  protected def durationIn(unit: TimeUnit): FiniteDuration = Duration(d, unit) match
    case f: FiniteDuration => f
    case _ => throw new IllegalArgumentException("Duration DSL not applicable to " + d)

final class IntMult(i: Int):
  def *(d: FiniteDuration): FiniteDuration = d * i.toLong
def IntMult(i: Int): IntMult = new IntMult(i)

final class LongMult(i: Long):
  def *(d: FiniteDuration): FiniteDuration = d * i
def LongMult(i: Long): LongMult = new LongMult(i)

final class DoubleMult(f: Double):
  def *(d: FiniteDuration): FiniteDuration = Duration.fromNanos((d.toNanos.toDouble * f).toLong)
def DoubleMult(f: Double): DoubleMult = new DoubleMult(f)

def pairIntToDuration(p: (Int, TimeUnit)): FiniteDuration = Duration(p._1.toLong, p._2)
def pairLongToDuration(p: (Long, TimeUnit)): FiniteDuration = Duration(p._1, p._2)
def durationToPair(d: FiniteDuration): (Long, TimeUnit) = (d.length, d.unit)

// For a compiler that searches no companions of type arguments; duration.* imports it there.
given Ordering[FiniteDuration] = FiniteDuration.FiniteDurationIsOrdered

extension (n: Int | Long)
  def nano: FiniteDuration = new FiniteDuration(durationLength(n), 1L)
  def nanos: FiniteDuration = new FiniteDuration(durationLength(n), 1L)
  def nanosecond: FiniteDuration = new FiniteDuration(durationLength(n), 1L)
  def nanoseconds: FiniteDuration = new FiniteDuration(durationLength(n), 1L)
  def micro: FiniteDuration = new FiniteDuration(durationLength(n), 1000L)
  def micros: FiniteDuration = new FiniteDuration(durationLength(n), 1000L)
  def microsecond: FiniteDuration = new FiniteDuration(durationLength(n), 1000L)
  def microseconds: FiniteDuration = new FiniteDuration(durationLength(n), 1000L)
  def milli: FiniteDuration = new FiniteDuration(durationLength(n), 1000000L)
  def millis: FiniteDuration = new FiniteDuration(durationLength(n), 1000000L)
  def millisecond: FiniteDuration = new FiniteDuration(durationLength(n), 1000000L)
  def milliseconds: FiniteDuration = new FiniteDuration(durationLength(n), 1000000L)
  def second: FiniteDuration = new FiniteDuration(durationLength(n), 1000000000L)
  def seconds: FiniteDuration = new FiniteDuration(durationLength(n), 1000000000L)
  def minute: FiniteDuration = new FiniteDuration(durationLength(n), 60000000000L)
  def minutes: FiniteDuration = new FiniteDuration(durationLength(n), 60000000000L)
  def hour: FiniteDuration = new FiniteDuration(durationLength(n), 3600000000000L)
  def hours: FiniteDuration = new FiniteDuration(durationLength(n), 3600000000000L)
  def day: FiniteDuration = new FiniteDuration(durationLength(n), 86400000000000L)
  def days: FiniteDuration = new FiniteDuration(durationLength(n), 86400000000000L)

@js("BigInt($0)")
@jvm("$0:L checkcast java/lang/Number invokevirtual java/lang/Number.longValue()J")
def durationLength(n: Int | Long): Long
