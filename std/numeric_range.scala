package scala.collection.immutable

// scala-library's `NumericRange`: the elements from `start` towards `end` by `step`, computed by
// the `Integral` (of `Long`, `Char`, `BigInt`, a `BigDecimal` as if integral), its length counted as
// scala-library counts it: through an `Int` range where the bounds fit, else in the type itself,
// failing past `Int.MaxValue` elements and on a zero step.
sealed class NumericRange[T](val start: T, val end: T, val step: T, val isInclusive: Boolean)(implicit num: Integral[T]) extends SeqOps[T, IndexedSeq, IndexedSeq[T]], IndexedSeq[T]:
  def buildCC[B](items: RawBuffer[B]): IndexedSeq[B] = Vector.wrap(items)
  override def collectionClassName: String = "NumericRange"
  override lazy val length: Int = NumericRange.count(start, end, step, isInclusive)
  override lazy val isEmpty: Boolean =
    (num.gt(start, end) && num.gt(step, num.zero)) || (num.lt(start, end) && num.lt(step, num.zero)) || (num.equiv(start, end) && !isInclusive)
  def by(newStep: T): NumericRange[T] = copy(start, end, newStep)
  def copy(start: T, end: T, step: T): NumericRange[T] = new NumericRange(start, end, step, isInclusive)
  def apply(idx: Int): T =
    if idx < 0 || idx >= length then throw new IndexOutOfBoundsException(s"$idx is out of bounds (min 0, max ${length - 1})")
    else num.plus(start, num.times(step, num.fromInt(idx)))
  def foreach[U](f: T => U): Unit =
    var count = 0
    var current = start
    while count < length do
      f(current)
      current = num.plus(current, step)
      count += 1
  override def iterator: Iterator[T] =
    var count = 0
    var current = start
    def next(): T =
      val x = current
      current = num.plus(current, step)
      count += 1
      x
    new FnIterator(() => count < length, () => next())
  override def head: T = if isEmpty then noSuchElement("head of empty NumericRange") else start
  override def last: T = if isEmpty then noSuchElement("last of empty NumericRange") else apply(length - 1)
  override def toString: String =
    val empty = if isEmpty then "empty " else ""
    val preposition = if isInclusive then "to" else "until"
    val stepped = if num.equiv(step, num.one) then "" else s" by $step"
    s"${empty}NumericRange $start $preposition $end$stepped"

object NumericRange:
  /** The exclusive range `start until end by step`. */
  def apply[T](start: T, end: T, step: T)(implicit num: Integral[T]): Exclusive[T] = new Exclusive(start, end, step)
  /** The inclusive range `start to end by step`. */
  def inclusive[T](start: T, end: T, step: T)(implicit num: Integral[T]): Inclusive[T] = new Inclusive(start, end, step)

  final class Inclusive[T](start: T, end: T, step: T)(implicit num: Integral[T]) extends NumericRange[T](start, end, step, true):
    override def copy(start: T, end: T, step: T): Inclusive[T] = NumericRange.inclusive(start, end, step)
    override def by(newStep: T): Inclusive[T] = copy(start, end, newStep)
    def exclusive: Exclusive[T] = NumericRange(start, end, step)

  final class Exclusive[T](start: T, end: T, step: T)(implicit num: Integral[T]) extends NumericRange[T](start, end, step, false):
    override def copy(start: T, end: T, step: T): Exclusive[T] = NumericRange(start, end, step)
    override def by(newStep: T): Exclusive[T] = copy(start, end, newStep)
    def inclusive: Inclusive[T] = NumericRange.inclusive(start, end, step)

  // scala-library's: a `BigDecimal` step that the bounds' precision loses fails the range.
  private def bigDecimalCheckUnderflow[T](start: T, end: T, step: T)(implicit num: Integral[T]): Unit =
    def fail(boundary: T): Unit =
      val msg = boundary match
        case bd: BigDecimal => s"Precision ${bd.mc.getPrecision}"
        case _ => "Precision"
      throw new IllegalArgumentException(s"$msg inadequate to represent steps of size $step near $boundary")
    if num.minus(num.plus(start, step), start) != step then fail(start)
    if num.minus(end, num.minus(end, step)) != step then fail(end)

  def count[T](start: T, end: T, step: T, isInclusive: Boolean)(implicit num: Integral[T]): Int =
    val zero = num.zero
    val upward = num.lt(start, end)
    val posStep = num.gt(step, zero)
    if step == zero then throw new IllegalArgumentException("step cannot be 0.")
    else if start == end then (if isInclusive then 1 else 0)
    else if upward != posStep then 0
    else
      // The bounds and the step tried as `Int`s one after another, each converted only once the one
      // before it fits, deferring to an `Int` range where all three do.
      val startint = num.toInt(start)
      if start == num.fromInt(startint) then
        val endint = num.toInt(end)
        if end == num.fromInt(endint) then
          val stepint = num.toInt(step)
          if step == num.fromInt(stepint) then
            return new Range(startint, endint, stepint, isInclusive).length
      if num.isInstanceOf[Numeric.BigDecimalAsIfIntegral.type] then bigDecimalCheckUnderflow(start, end, step)
      val one = num.one
      val limit = num.fromInt(Int.MaxValue)
      def check(t: T): T = if num.gt(t, limit) then throw new IllegalArgumentException("More than Int.MaxValue elements.") else t
      val startside = num.sign(start)
      val endside = num.sign(end)
      num.toInt(
        if num.gteq(num.times(startside, endside), zero) then
          val diff = num.minus(end, start)
          val quotient = check(num.quot(diff, step))
          val remainder = num.minus(diff, num.times(quotient, step))
          if !isInclusive && zero == remainder then quotient else check(num.plus(quotient, one))
        else
          // Through a point beside zero, in three pieces, as scala-library steps there.
          val negone = num.fromInt(-1)
          val startlim = if posStep then negone else one
          val startdiff = num.minus(if num.lteq(num.abs(start), num.abs(startlim)) then start else startlim, start)
          val startq = check(num.quot(startdiff, step))
          val waypointA = if startq == zero then start else num.plus(start, num.times(startq, step))
          val waypointB = num.plus(waypointA, step)
          check {
            if num.lt(waypointB, end) != upward then
              if isInclusive && waypointB == end then num.plus(startq, num.fromInt(2))
              else num.plus(startq, one)
            else
              val enddiff = num.minus(end, waypointB)
              val endq = check(num.quot(enddiff, step))
              val last = if endq == zero then waypointB else num.plus(waypointB, num.times(endq, step))
              num.plus(startq, num.plus(endq, if !isInclusive && last == end then one else num.fromInt(2)))
          }
      )
