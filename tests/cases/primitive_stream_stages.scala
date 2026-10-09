// The primitive streams' stages as the JDK's `IntPipeline`, `LongPipeline` and `DoublePipeline` run
// them: lazily, each element through every stage in turn (`peek` shows the order), a sized source's
// `count` running none, `limit` stopping an endless `iterate`, and their terminal operations; and
// an exception's suppressed exceptions (`addSuppressed`), kept in the order added.
import java.util.stream.{DoubleStream, IntStream, LongStream}

object Main:
  def main(args: Array[String]): Unit =
    println(IntStream.of(1, 2, 3, 4).filter(x => x % 2 == 0).map(x => x * 10).sum())
    println(IntStream.range(0, 5).mapToObj(i => "i" + i).collect(java.util.stream.Collectors.joining(",")))
    println(IntStream.rangeClosed(1, 4).boxed().collect(java.util.stream.Collectors.toList()))
    IntStream.of(3, 1, 2).peek(x => println("peek " + x)).filter(x => x > 1).forEach(x => println("got " + x))
    println(IntStream.of(5, 3, 5, 1).distinct().sorted().boxed().collect(java.util.stream.Collectors.toList()))
    println(IntStream.iterate(1, x => x * 2).limit(5).sum())
    println(IntStream.iterate(1, x => x < 50, x => x * 3).count())
    println(IntStream.range(0, 10).skip(7).reduce(0, (a, b) => a + b))
    println(IntStream.range(0, 4).reduce((a, b) => a * 10 + b).getAsInt())
    println(IntStream.of(1, 2, 3).takeWhile(x => x < 3).count())
    println(IntStream.of(1, 2, 3, 1).dropWhile(x => x < 3).boxed().collect(java.util.stream.Collectors.toList()))
    println(IntStream.of(4, 6).anyMatch(x => x % 2 == 1))
    println(IntStream.of(4, 6).allMatch(x => x % 2 == 0))
    println(IntStream.of(4, 6).noneMatch(x => x > 5))
    println(IntStream.range(3, 9).filter(x => x % 4 == 0).findFirst().getAsInt())
    println(IntStream.empty().findFirst().isPresent())
    println(IntStream.of(1, 2).flatMap(x => IntStream.of(x, x)).count())
    println(IntStream.of(1, 2).asLongStream().map(x => x * 3000000000L).sum())
    println(IntStream.of(7).mapToDouble(x => x / 2.0).sum())
    var n = 0
    println(IntStream.range(0, 4).map(x => { n += 1; x }).count())
    println(n)
    val sb = IntStream.of(1, 2).collect(() => new StringBuilder, (b, x) => { b.append(x); () }, (a, b) => { a.append(b); () })
    println(sb)
    println(LongStream.rangeClosed(1L, 5L).filter(x => x > 2L).map(x => x * x).sum())
    println(LongStream.of(9L, 4L).mapToInt(x => x.toInt).max().getAsInt())
    println(LongStream.iterate(1L, x => x * 10L).limit(3).boxed().collect(java.util.stream.Collectors.toList()))
    println(DoubleStream.of(1.5, 2.5).map(x => x * 2).sum().toInt)
    println(DoubleStream.of(2.0, 1.0, 2.0).distinct().count())
    println(DoubleStream.of(1.0, 4.0).mapToLong(x => x.toLong).sum())
    println(DoubleStream.of(0.5, 3.0).filter(x => x > 1.0).findFirst().getAsDouble().toInt)
    val e = new Exception("e")
    println(e.getSuppressed.length)
    e.addSuppressed(new RuntimeException("s1"))
    e.addSuppressed(new IllegalStateException("s2"))
    println(e.getSuppressed.map(_.getMessage).mkString(","))
    try e.addSuppressed(e) catch case x: IllegalArgumentException => println(x.getMessage)
    try e.addSuppressed(null) catch case x: NullPointerException => println(x.getMessage)
