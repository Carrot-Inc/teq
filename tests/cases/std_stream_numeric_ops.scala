// scala-library's deprecated `Stream` (a strict head and a lazy, remembered tail; `#::` taking its tail by
// name), `Traversable` and `TraversableOnce`, the operators `Numeric` and `Ordering` give through
// `mkNumericOps` and `mkOrderingOps`, and `TreeMap.newBuilder`: what scalacheck's and cats-laws' bodies
// call.
import scala.collection.immutable.TreeMap

@deprecated("uses Stream", "") object Streams:
  var made = 0
  def count(n: Int): Stream[Int] =
    made += 1
    n #:: count(n + 1)
  def halves(n: Int): Stream[Int] = if n == 0 then Stream.empty else n #:: halves(n / 2)

def clamp[T](x: T, lo: T, hi: T)(implicit num: Numeric[T]): T =
  import num.*
  if x < lo then lo else if x > hi then hi else x * one

@deprecated("uses Stream", "") @main def run(): Unit =
  val s = Streams.count(1)
  println(Streams.made)
  println(s.take(3).toList)
  println(Streams.made)
  println(Streams.halves(40).toList)
  println(Stream(1, 2, 3).map(_ * 2).filter(_ > 2).toList)
  println(Stream.from(5).dropWhile(_ < 8).head)
  println((Stream(1, 2) ++ Stream(3)).toList)
  println(List(4, 5).toStream.head)
  val t: Traversable[Int] = Vector(1, 2, 3)
  val o: TraversableOnce[Int] = List(4)
  println(t.sum + o.iterator.sum)
  println(clamp(7, 0, 5) + clamp(-1, 0, 5))
  println(clamp(2.5, 0.0, 5.0))
  val num = Numeric[Long]
  println(num.mkNumericOps(4L) - 1L)
  println(num.mkOrderingOps(4L) < 9L)
  val b = TreeMap.newBuilder[Int, String]
  b += 3 -> "c"
  b += 1 -> "a"
  println(b.result())
