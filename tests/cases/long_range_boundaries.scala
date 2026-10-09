// The second code pass's `long_range_boundaries` program.
import java.util.stream.LongStream
@main def run(): Unit =
  val pairs = List((Long.MinValue, -1L), (Long.MinValue, 0L), (-1L, Long.MaxValue), (0L, Long.MaxValue))
  pairs.foreach { (lo, hi) =>
    val s = LongStream.rangeClosed(lo, hi).spliterator()
    println(s.estimateSize())
    println(s.hasCharacteristics(java.util.Spliterator.SIZED))
    println(LongStream.rangeClosed(lo, hi).skip(2).limit(2).toArray().mkString(","))
  }
