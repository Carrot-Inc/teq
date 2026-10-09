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

sealed trait Try[+T]:
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
    if p(v) then Success(v) else Failure(new NoSuchElementException("Predicate does not hold for " + v.toString))
  def withFilter(p: T => Boolean): Try[T] = filter(p)
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

// java.util.Random: the same numbers for the same seed.
trait RandomGenerator:
  def next(bits: Int): Int
  def setSeed(seed: Long): Unit
  // Without an argument the default stands for "no bound".
  def nextInt(bound: Int = -2147483648): Int =
    if bound == -2147483648 then next(32)
    else if bound <= 0 then illegalArgument("bound must be positive")
    else if (bound & -bound) == bound then ((bound.toLong * next(31).toLong) >> 31).toInt
    else
      var bits = next(31)
      var value = bits % bound
      while bits - value + (bound - 1) < 0 do
        bits = next(31)
        value = bits % bound
      value
  def nextLong(): Long = (next(32).toLong << 32) + next(32).toLong
  def nextDouble(): Double = ((next(26).toLong << 27) + next(27).toLong).toDouble / 9007199254740992.0
  def nextBoolean(): Boolean = next(1) != 0
  def nextPrintableChar(): Char = (nextInt(94) + 33).toChar
  def between(minInclusive: Int, maxExclusive: Int): Int = nextInt(maxExclusive - minInclusive) + minInclusive
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

def scrambleSeed(seed: Long): Long = (seed ^ 0x5DEECE66DL) & 0xFFFFFFFFFFFFL

@js("BigInt(Math.floor(Math.random() * 281474976710656))")
@jvm("invokestatic java/lang/System.nanoTime()J")
def freshSeed(): Long

final class Random(init: Long = freshSeed()) extends RandomGenerator:
  private var seed = scrambleSeed(init)
  def setSeed(value: Long): Unit = seed = scrambleSeed(value)
  def next(bits: Int): Int =
    seed = (seed * 0x5DEECE66DL + 0xBL) & 0xFFFFFFFFFFFFL
    (seed >>> (48 - bits)).toInt

object Random extends RandomGenerator:
  private var seed = scrambleSeed(freshSeed())
  def setSeed(value: Long): Unit = seed = scrambleSeed(value)
  def next(bits: Int): Int =
    seed = (seed * 0x5DEECE66DL + 0xBL) & 0xFFFFFFFFFFFFL
    (seed >>> (48 - bits)).toInt

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
