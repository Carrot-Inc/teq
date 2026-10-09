package scala

// A `java.util.Comparator` too, as scala-library's is: a library passes an `Ordering` where a
// Java collection takes one (`Collections.sort(list, ordering)`).
trait Ordering[T] extends java.util.Comparator[T]:
  def compare(a: T, b: T): Int
  def lt(a: T, b: T): Boolean = compare(a, b) < 0
  def gt(a: T, b: T): Boolean = compare(a, b) > 0
  def lteq(a: T, b: T): Boolean = compare(a, b) <= 0
  def gteq(a: T, b: T): Boolean = compare(a, b) >= 0
  def equiv(a: T, b: T): Boolean = compare(a, b) == 0
  def max[U <: T](a: U, b: U): U = if compare(a, b) >= 0 then a else b
  def min[U <: T](a: U, b: U): U = if compare(a, b) <= 0 then a else b
  def reverse: Ordering[T] = Ordering.fromCompare((a, b) => compare(b, a))
  def on[U](f: U => T): Ordering[U] = Ordering.fromCompare((a, b) => compare(f(a), f(b)))
  def orElse(other: Ordering[T]): Ordering[T] =
    Ordering.fromCompare: (a, b) =>
      val c = compare(a, b)
      if c != 0 then c else other.compare(a, b)
  def orElseBy[S](f: T => S)(implicit ord: Ordering[S]): Ordering[T] = orElse(ord.on(f))

final class FunctionOrdering[T](f: (T, T) => Int) extends Ordering[T]:
  def compare(a: T, b: T): Int = f(a, b)

trait Ordered[T] extends java.lang.Comparable[T]:
  def compare(that: T): Int
  def compareTo(that: T): Int = compare(that)
  def <(that: T): Boolean = compare(that) < 0
  def >(that: T): Boolean = compare(that) > 0
  def <=(that: T): Boolean = compare(that) <= 0
  def >=(that: T): Boolean = compare(that) >= 0

@js("($0 < $1 ? -1 : $0 > $1 ? 1 : 0)")
@jvm("$0:L checkcast java/lang/Comparable $1:L invokeinterface java/lang/Comparable.compareTo(Ljava/lang/Object;)I")
def comparePrimitives[T](a: T, b: T): Int

// java.lang.Double.compare: -0.0 sorts before 0.0 and NaN after everything else.
@js("$compareDoubles($0, $1)")
@jvm("invokestatic java/lang/Double.compare(DD)I")
def compareDoubles(a: Double, b: Double): Int

object Ordering:
  def apply[T](implicit ord: Ordering[T]): Ordering[T] = ord
  def fromCompare[T](f: (T, T) => Int): Ordering[T] = FunctionOrdering(f)
  def fromLessThan[T](lt: (T, T) => Boolean): Ordering[T] =
    FunctionOrdering((a, b) => if lt(a, b) then -1 else if lt(b, a) then 1 else 0)
  def by[T, S](f: T => S)(implicit ord: Ordering[S]): Ordering[T] =
    FunctionOrdering((a, b) => ord.compare(f(a), f(b)))

  given Int: Ordering[Int] = FunctionOrdering((a, b) => comparePrimitives(a, b))
  given Long: Ordering[Long] = FunctionOrdering((a, b) => comparePrimitives(a, b))
  given String: Ordering[String] = FunctionOrdering((a, b) => comparePrimitives(a, b))
  given Char: Ordering[Char] = FunctionOrdering((a, b) => comparePrimitives(a, b))
  given Boolean: Ordering[Boolean] = FunctionOrdering((a, b) => comparePrimitives(a, b))
  given Unit: Ordering[Unit] = FunctionOrdering((a, b) => 0)
  // scala-library's names for the primitive instances, which a library matches on to pick an
  // array sort (zio's `Chunk.sorted`): the givens of `Byte`, `Short` and `Float` live in their
  // companions and are these values, and the `Double` and `Float` orderings are one instance
  // each here, `compare` being Java's on both the IEEE and the total ordering.
  val Byte: Ordering[scala.Byte] = FunctionOrdering((a, b) => comparePrimitives(a, b))
  val Short: Ordering[scala.Short] = FunctionOrdering((a, b) => comparePrimitives(a, b))
  given DeprecatedDoubleOrdering: Ordering[scala.Double] = FunctionOrdering((a, b) => compareDoubles(a, b))
  val DeprecatedFloatOrdering: Ordering[scala.Float] = FunctionOrdering((a, b) => compareDoubles(a, b))
  object Double:
    val IeeeOrdering: Ordering[scala.Double] = DeprecatedDoubleOrdering
    val TotalOrdering: Ordering[scala.Double] = DeprecatedDoubleOrdering
  object Float:
    val IeeeOrdering: Ordering[scala.Float] = DeprecatedFloatOrdering
    val TotalOrdering: Ordering[scala.Float] = DeprecatedFloatOrdering

  def ordered[T <: Ordered[T]]: Ordering[T] = FunctionOrdering((a, b) => a.compare(b))

  // scala-library's `Ordering.ordered`, for a class that is `Comparable` to a supertype of
  // itself (`LocalDate` compares as a `ChronoLocalDate`).
  given comparable[T <: java.lang.Comparable[?]]: Ordering[T] = FunctionOrdering((a, b) => a.asInstanceOf[java.lang.Comparable[Any]].compareTo(b))

  given Option[T](implicit ord: Ordering[T]): Ordering[Option[T]] =
    FunctionOrdering: (x, y) =>
      (x, y) match
        case (Some(a), Some(b)) => ord.compare(a, b)
        case (None, None) => 0
        case (None, _) => -1
        case _ => 1

  given Tuple2[A, B](using oa: Ordering[A], ob: Ordering[B]): Ordering[(A, B)] =
    FunctionOrdering: (x, y) =>
      val c = oa.compare(x._1, y._1)
      if c != 0 then c else ob.compare(x._2, y._2)

  given Tuple3[A, B, C](using oa: Ordering[A], ob: Ordering[B], oc: Ordering[C]): Ordering[(A, B, C)] =
    FunctionOrdering: (x, y) =>
      val c = oa.compare(x._1, y._1)
      if c != 0 then c else Tuple2(using ob, oc).compare((x._2, x._3), (y._2, y._3))

  given Tuple4[A, B, C, D](using oa: Ordering[A], ob: Ordering[B], oc: Ordering[C], od: Ordering[D]): Ordering[(A, B, C, D)] =
    FunctionOrdering: (x, y) =>
      val c = oa.compare(x._1, y._1)
      if c != 0 then c else Tuple3(using ob, oc, od).compare((x._2, x._3, x._4), (y._2, y._3, y._4))

  given Tuple5[A, B, C, D, E](using oa: Ordering[A], ob: Ordering[B], oc: Ordering[C], od: Ordering[D], oe: Ordering[E]): Ordering[(A, B, C, D, E)] =
    FunctionOrdering: (x, y) =>
      val c = oa.compare(x._1, y._1)
      if c != 0 then c else Tuple4(using ob, oc, od, oe).compare((x._2, x._3, x._4, x._5), (y._2, y._3, y._4, y._5))

  // Implicits.* imports the operators as members and Implicits.infixOrderingOps as those of a given.
  trait InfixOrderingOps:
    extension [T](x: T)(using ord: Ordering[T])
      def <(y: T): Boolean = ord.lt(x, y)
      def >(y: T): Boolean = ord.gt(x, y)
      def <=(y: T): Boolean = ord.lteq(x, y)
      def >=(y: T): Boolean = ord.gteq(x, y)
      def equiv(y: T): Boolean = ord.equiv(x, y)
      def max(y: T): T = ord.max(x, y)
      def min(y: T): T = ord.min(x, y)

  object Implicits extends InfixOrderingOps:
    given infixOrderingOps: InfixOrderingOps = this
    // Lexicographic, a shorter prefix first, as scala-library's.
    implicit def seqOrdering[CC[X] <: Seq[X], T](implicit ord: Ordering[T]): Ordering[CC[T]] =
      new Ordering[CC[T]]:
        def compare(x: CC[T], y: CC[T]): Int =
          val xs = x.iterator
          val ys = y.iterator
          var res = 0
          while res == 0 && xs.hasNext && ys.hasNext do res = ord.compare(xs.next(), ys.next())
          if res != 0 then res else java.lang.Boolean.compare(xs.hasNext, ys.hasNext)

trait Numeric[T]:
  def zero: T
  def one: T
  def plus(a: T, b: T): T
  def minus(a: T, b: T): T
  def times(a: T, b: T): T
  def fromInt(x: Int): T
  def toInt(x: T): Int
  def toLong(x: T): Long
  def toDouble(x: T): Double
  def compare(a: T, b: T): Int
  def negate(x: T): T = minus(zero, x)
  def abs(x: T): T = if compare(x, zero) < 0 then negate(x) else x
  def sign(x: T): T =
    val c = compare(x, zero)
    if c < 0 then negate(one) else if c > 0 then one else zero
  def signum(x: T): Int = compare(x, zero)
  def lt(a: T, b: T): Boolean = compare(a, b) < 0
  def gt(a: T, b: T): Boolean = compare(a, b) > 0
  def lteq(a: T, b: T): Boolean = compare(a, b) <= 0
  def gteq(a: T, b: T): Boolean = compare(a, b) >= 0
  def equiv(a: T, b: T): Boolean = compare(a, b) == 0
  def max[U <: T](a: U, b: U): U = if compare(a, b) >= 0 then a else b
  def min[U <: T](a: U, b: U): U = if compare(a, b) <= 0 then a else b

trait Integral[T] extends Numeric[T]:
  def quot(a: T, b: T): T
  def rem(a: T, b: T): T

trait Fractional[T] extends Numeric[T]:
  def div(a: T, b: T): T

object Numeric:
  def apply[T](implicit num: Numeric[T]): Numeric[T] = num

  given IntIsIntegral: Integral[Int] with
    def zero: Int = 0
    def one: Int = 1
    def plus(a: Int, b: Int): Int = a + b
    def minus(a: Int, b: Int): Int = a - b
    def times(a: Int, b: Int): Int = a * b
    def quot(a: Int, b: Int): Int = a / b
    def rem(a: Int, b: Int): Int = a % b
    def fromInt(x: Int): Int = x
    def toInt(x: Int): Int = x
    def toLong(x: Int): Long = x.toLong
    def toDouble(x: Int): Double = x.toDouble
    def compare(a: Int, b: Int): Int = comparePrimitives(a, b)

  given LongIsIntegral: Integral[Long] with
    def zero: Long = 0L
    def one: Long = 1L
    def plus(a: Long, b: Long): Long = a + b
    def minus(a: Long, b: Long): Long = a - b
    def times(a: Long, b: Long): Long = a * b
    def quot(a: Long, b: Long): Long = a / b
    def rem(a: Long, b: Long): Long = a % b
    def fromInt(x: Int): Long = x.toLong
    def toInt(x: Long): Int = x.toInt
    def toLong(x: Long): Long = x
    def toDouble(x: Long): Double = x.toDouble
    def compare(a: Long, b: Long): Int = comparePrimitives(a, b)

  given DoubleIsFractional: Fractional[Double] with
    def zero: Double = 0.0
    def one: Double = 1.0
    def plus(a: Double, b: Double): Double = a + b
    def minus(a: Double, b: Double): Double = a - b
    def times(a: Double, b: Double): Double = a * b
    def div(a: Double, b: Double): Double = a / b
    def fromInt(x: Int): Double = x.toDouble
    def toInt(x: Double): Int = x.toInt
    def toLong(x: Double): Long = x.toLong
    def toDouble(x: Double): Double = x
    def compare(a: Double, b: Double): Int = compareDoubles(a, b)

  trait InfixNumericOps:
    extension [T](x: T)(using num: Numeric[T])
      def +(y: T): T = num.plus(x, y)
      def -(y: T): T = num.minus(x, y)
      def *(y: T): T = num.times(x, y)
      def unary_- : T = num.negate(x)

  object Implicits extends InfixNumericOps:
    given infixNumericOps: InfixNumericOps = this

object Integral:
  def apply[T](using num: Integral[T]): Integral[T] = num

object Fractional:
  def apply[T](using num: Fractional[T]): Fractional[T] = num

