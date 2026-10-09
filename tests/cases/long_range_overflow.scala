// `LongStream.range` and `rangeClosed` of more than `Long.MaxValue` numbers: two ranges split at
// the unsigned middle and concatenated, as the JDK's, each piece sized and the whole not
// (`Streams.ConcatSpliterator`: no `SIZED`, the estimate `Long.MaxValue`); and the primitive
// streams' `concat`, which closes both.
import java.util.stream.{DoubleStream, IntStream, LongStream}
import java.util.Spliterator

@main def run(): Unit =
  println(LongStream.rangeClosed(Long.MinValue, Long.MaxValue).limit(3).count())
  println(LongStream.rangeClosed(Long.MinValue, Long.MaxValue).spliterator().estimateSize())
  println(LongStream.range(Long.MinValue, Long.MaxValue).spliterator().hasCharacteristics(Spliterator.SIZED))
  println(LongStream.rangeClosed(Long.MinValue, Long.MaxValue).limit(3).toArray().mkString(","))
  println(LongStream.rangeClosed(-2, Long.MaxValue).spliterator().estimateSize())
  println(LongStream.rangeClosed(Long.MaxValue - 2, Long.MaxValue).sum())
  println(LongStream.range(0, Long.MaxValue).spliterator().hasCharacteristics(Spliterator.SIZED))
  println(java.lang.Long.divideUnsigned(-1L, 2) + " " + java.lang.Long.divideUnsigned(-1L, -2L) + " " + java.lang.Long.divideUnsigned(10L, 3L))
  val closed = new StringBuilder
  val ints = IntStream.concat(IntStream.of(1, 2).onClose(() => closed.append("a")), IntStream.rangeClosed(3, 4).onClose(() => closed.append("b")))
  println(ints.spliterator().estimateSize())
  val more = IntStream.concat(IntStream.of(1, 2).onClose(() => closed.append("c")), IntStream.rangeClosed(3, 4).onClose(() => closed.append("d")))
  println(more.map(_ * 10).boxed().toList)
  more.close()
  println(closed)
  println(DoubleStream.concat(DoubleStream.of(0.5), DoubleStream.of(1.5, 2.5)).sum())
  println(LongStream.concat(LongStream.of(1L), LongStream.empty()).count())
