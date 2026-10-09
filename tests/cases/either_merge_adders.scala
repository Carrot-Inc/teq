// `Either.MergeableEither`, which compiled library bodies call for `merge`, `LongAdder` as zio's
// supervisor counts with it, and a `ConcurrentHashMap` copied from another.
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.LongAdder

object Main:
  def main(args: Array[String]): Unit =
    val l: Either[Int, Int] = Left(3)
    val r: Either[Int, Int] = Right(4)
    println(Either.MergeableEither(l).merge + r.merge)
    val adder = new LongAdder()
    adder.increment()
    adder.add(10L)
    adder.decrement()
    println(adder.sum())
    println(adder)
    println(adder.sumThenReset())
    println(adder.sum())
    val m = new ConcurrentHashMap[String, Int]()
    m.put("a", 1)
    val copy = new ConcurrentHashMap[String, Int](m)
    copy.put("b", 2)
    println(s"${m.get("a")} ${m.containsKey("b")} ${copy.get("a")} ${copy.get("b")}")
