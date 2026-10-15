package scala.util

import scala.util.control.NonFatal

// A caught value that is no Throwable, wrapped as `js.JavaScriptException`; kept for the code
// that named it before the exception classes existed.
object JsError:
  def wrap(error: Any): Throwable = error match
    case t: Throwable => t
    case _ => js.JavaScriptException(error)
  def unwrap(t: Throwable): Any = t match
    case js.JavaScriptException(e) => e
    case _ => t

// `Product` and `Serializable`, as scala-library's `Try`.
sealed trait Try[+T] extends Product, java.io.Serializable:
  def isSuccess: Boolean = this match
    case Success(_) => true
    case Failure(_) => false
  def isFailure: Boolean = !isSuccess
  def get: T = this match
    case Success(v) => v
    case Failure(e) => throw e
  def getOrElse[U >: T](default: => U): U = this match
    case Success(v) => v
    case Failure(_) => default
  def orElse[U >: T](alternative: => Try[U]): Try[U] = this match
    case Success(_) => this
    case Failure(_) => Try(alternative).flatten
  def toOption: Option[T] = this match
    case Success(v) => Some(v)
    case Failure(_) => None
  def toEither: Either[Throwable, T] = this match
    case Success(v) => Right(v)
    case Failure(e) => Left(e)
  def map[U](f: T => U): Try[U] = this match
    case Success(v) => Try(f(v))
    case Failure(e) => Failure(e)
  def flatMap[U](f: T => Try[U]): Try[U] = this match
    case Success(v) => Try(f(v)).flatten
    case Failure(e) => Failure(e)
  def filter(p: T => Boolean): Try[T] = flatMap: v =>
    if p(v) then Success(v) else Failure(new NoSuchElementException("Predicate does not hold for " + v))
  def withFilter(p: T => Boolean): Try[T] = filter(p)
  def collect[U](pf: PartialFunction[T, U]): Try[U] = flatMap: v =>
    Try(if pf.isDefinedAt(v) then pf(v) else throw new NoSuchElementException("Predicate does not hold for " + v))
  def foreach[U](f: T => U): Unit = this match
    case Success(v) =>
      f(v)
      ()
    case Failure(_) => ()
  def fold[U](onFailure: Throwable => U, onSuccess: T => U): U = this match
    case Success(v) => Try(onSuccess(v)) match
      case Success(u) => u
      case Failure(e) => onFailure(e)
    case Failure(e) => onFailure(e)
  def transform[U](onSuccess: T => Try[U], onFailure: Throwable => Try[U]): Try[U] = this match
    case Success(v) => Try(onSuccess(v)).flatten
    case Failure(e) => Try(onFailure(e)).flatten
  def recover[U >: T](pf: PartialFunction[Throwable, U]): Try[U] = this match
    case Failure(e) if pf.isDefinedAt(e) => Try(pf(e))
    case _ => this
  def recoverWith[U >: T](pf: PartialFunction[Throwable, Try[U]]): Try[U] = this match
    case Failure(e) if pf.isDefinedAt(e) => Try(pf(e)).flatten
    case _ => this
  def failed: Try[Throwable] = this match
    case Success(_) => Failure(new UnsupportedOperationException("Success.failed"))
    case Failure(e) => Success(e)

final case class Success[+T](value: T) extends Try[T]
final case class Failure[+T](exception: Throwable) extends Try[T]

object Try:
  def apply[T](body: => T): Try[T] =
    try Success(body)
    catch case NonFatal(e) => Failure(e)

  extension [T](t: Try[T])
    def flatten[U](implicit ev: T <:< Try[U]): Try[U] = t match
      case Success(inner) => ev(inner)
      case Failure(e) => Failure(e)

object chaining:
  extension [A](self: A)
    def pipe[B](f: A => B): B = f(self)
    def tap[U](f: A => U): A =
      f(self)
      self

// scala-library's: a wrapper of a `java.util.Random`, whose numbers it gives, the same for the
// same seed as the JDK's.
class Random(val self: java.util.Random):
  def this(seed: Long) = this(new java.util.Random(seed))
  def this(seed: Int) = this(seed.toLong)
  def this() = this(new java.util.Random())
  def nextBoolean(): Boolean = self.nextBoolean()
  def nextBytes(bytes: Array[Byte]): Unit = self.nextBytes(bytes)
  def nextDouble(): Double = self.nextDouble()
  def nextFloat(): Float = self.nextFloat()
  def nextGaussian(): Double = self.nextGaussian()
  def nextInt(): Int = self.nextInt()
  def nextInt(n: Int): Int = self.nextInt(n)
  def between(minInclusive: Int, maxExclusive: Int): Int =
    require(minInclusive < maxExclusive, "Invalid bounds")
    val difference = maxExclusive - minInclusive
    if difference >= 0 then nextInt(difference) + minInclusive
    else
      var n = nextInt()
      while n < minInclusive || n >= maxExclusive do n = nextInt()
      n
  def between(minInclusive: Long, maxExclusive: Long): Long =
    require(minInclusive < maxExclusive, "Invalid bounds")
    val difference = maxExclusive - minInclusive
    if difference >= 0 then nextLong(difference) + minInclusive
    else
      var n = nextLong()
      while n < minInclusive || n >= maxExclusive do n = nextLong()
      n
  def between(minInclusive: Double, maxExclusive: Double): Double =
    require(minInclusive < maxExclusive, "Invalid bounds")
    val next = nextDouble() * (maxExclusive - minInclusive) + minInclusive
    if next < maxExclusive then next else java.lang.Math.nextDown(maxExclusive)
  def between(minInclusive: Float, maxExclusive: Float): Float =
    require(minInclusive < maxExclusive, "Invalid bounds")
    val next = nextFloat() * (maxExclusive - minInclusive) + minInclusive
    if next < maxExclusive then next
    // scala-library's `Math.nextAfter(maxExclusive, Float.NegativeInfinity)`: the float below.
    else if maxExclusive == 0.0f then -java.lang.Float.MIN_VALUE
    else java.lang.Float.intBitsToFloat(java.lang.Float.floatToIntBits(maxExclusive) + (if maxExclusive > 0.0f then -1 else 1))
  def nextLong(): Long = self.nextLong()
  def nextLong(n: Long): Long =
    require(n > 0, "n must be positive")
    var offset = 0L
    var rest = n
    while rest >= Int.MaxValue do
      val bits = nextInt(2)
      val half = rest >>> 1
      val next = if (bits & 2) == 0 then half else rest - half
      if (bits & 1) == 0 then offset += rest - next
      rest = next
    offset + nextInt(rest.toInt)
  def nextPrintableChar(): Char = (self.nextInt(127 - 33) + 33).toChar
  /** scala-library's: `length` characters, each drawn from the surrogate-free range as `nextString` draws them. */
  def nextString(length: Int): String =
    val sb = new StringBuilder
    var i = 0
    while i < length do
      sb.append((self.nextInt(0xD800 - 1) + 1).toChar)
      i += 1
    sb.toString
  /** scala-library's: an endless stream of letters and digits, each drawn as it is read. */
  def alphanumeric: LazyList[Char] =
    val chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
    LazyList.continually(chars.charAt(self.nextInt(chars.length)))
  def setSeed(seed: Long): Unit = self.setSeed(seed)
  def shuffle[A, CC[_]](xs: IterableOps[A, CC, Any]): CC[A] =
    val items = rawItems(xs)
    var n = items.length
    while n >= 2 do
      val k = nextInt(n)
      val last = items(n - 1)
      items(n - 1) = items(k)
      items(k) = last
      n -= 1
    xs.buildCC(items)

object Random extends Random:
  implicit def javaRandomToRandom(r: java.util.Random): Random = new Random(r)

// scala-library's `Sorting`: sorts an array in place by an ordering; `stableSort` keeps equal
// elements in their order, as the std's sort does.
object Sorting:
  def stableSort[K](a: Array[K])(implicit ord: Ordering[K]): Unit =
    val sorted = a.toList.sorted(ord)
    var i = 0
    sorted.foreach { x =>
      a(i) = x
      i += 1
    }
  def stableSort[K](a: Array[K], lt: (K, K) => Boolean): Unit = stableSort(a)(Ordering.fromLessThan(lt))
  def quickSort[K](a: Array[K])(implicit ord: Ordering[K]): Unit = stableSort(a)(ord)

// scala-library's: a value bound for the extent of a thunk, kept in a thread-local, whose one slot
// on JavaScript is the program's one thread's.
class DynamicVariable[T](init: T):
  private val tl = new InheritableThreadLocal[T]:
    override protected def initialValue(): T = init
  def value: T = tl.get()
  def withValue[S](newval: T)(thunk: => S): S =
    val oldval = value
    tl.set(newval)
    try thunk
    finally tl.set(oldval)
  def value_=(newval: T): Unit = tl.set(newval)
  override def toString: String = "DynamicVariable(" + value + ")"
