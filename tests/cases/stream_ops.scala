// `java.util.stream` over the sources every platform has (`Stream.of`, `empty`, `iterate`,
// `generate`, `builder`, a collection's `stream`), as the JDK runs it: each member of `Stream`
// and `BaseStream`, the primitive streams' terminal operations, `Collectors`, `Collector` and
// `Optional`; when the functions run (nothing before the terminal operation, one element at a
// time through the stages, `count` of a sized pipeline running none), early termination, a
// stage used once, the exceptions of a null function and of a second use, and the close
// handlers. The values printed are platform-neutral: no whole doubles, and maps and sets whose
// keys the JDK's hash order and insertion order put alike.
import java.util.stream.{Collector, Collectors, DoubleStream, IntStream, LongStream, Stream}
import java.util.{ArrayList, Optional, OptionalDouble, OptionalInt, OptionalLong}
import java.util.function.{BiConsumer, BinaryOperator, Function, IntConsumer, Supplier}

object Main:
  // An element whose natural order counts its comparisons.
  var comparisons = 0
  final class Item(val n: Int) extends Comparable[Item]:
    def compareTo(that: Item): Int =
      comparisons += 1
      n - that.n
    override def toString: String = "i" + n

  val log = new StringBuilder
  def note(s: Any): Unit = log.append(s).append(' ')

  // A collector's characteristics telling each question asked of them.
  final class AskedTraits(traits: Collector.Characteristics*) extends java.util.HashSet[Collector.Characteristics]():
    traits.foreach(t => add(t))
    override def contains(o: Any): Boolean =
      note("contains:" + o)
      super.contains(o)
  final class Counted(traits: java.util.Set[Collector.Characteristics]) extends Collector[String, ArrayList[String], Int]:
    def supplier(): Supplier[ArrayList[String]] = () => { note("supply"); new ArrayList[String]() }
    def accumulator(): BiConsumer[ArrayList[String], String] = (l, x) => { note("acc"); l.add(x); () }
    def combiner(): BinaryOperator[ArrayList[String]] = (a, b) => a
    def finisher(): Function[ArrayList[String], Int] = l => l.size()
    def characteristics(): java.util.Set[Collector.Characteristics] = traits

  def sortedList(): ArrayList[String] =
    val list = new ArrayList[String]()
    list.add("a")
    list.add("b")
    list
  def show(name: String)(f: => Any): Unit =
    log.clear()
    val result =
      try String.valueOf(f)
      catch case e: Exception => e.getClass.getName + ": " + e.getMessage
    val seen = log.toString.trim
    println(name + ": " + result + (if seen.isEmpty then "" else "  [" + seen + "]"))

  def main(args: Array[String]): Unit =
    optionals()
    collectors()
    members()
    primitives()
    laziness()
    lifecycle()
    sources()

  def optionals(): Unit =
    show("of")(Optional.of("a"))
    show("of null")(Optional.of(null))
    show("ofNullable null")(Optional.ofNullable(null))
    show("empty")(Optional.empty[String]())
    show("get empty")(Optional.empty[String]().get())
    show("orElseThrow empty")(Optional.empty[String]().orElseThrow())
    show("orElseThrow supplier")(Optional.empty[String]().orElseThrow(() => new IllegalArgumentException("none")))
    show("present")(s"${Optional.of("a").isPresent()} ${Optional.of("a").isEmpty()} ${Optional.empty().isPresent()}")
    show("map")(Optional.of("ab").map[Int](s => s.length))
    show("map to null")(Optional.of("ab").map[String](s => null))
    show("map null on empty")(Optional.empty[String]().map[String](null))
    show("flatMap")(Optional.of("ab").flatMap[Int](s => Optional.of(s.length)))
    show("flatMap to null")(Optional.of("ab").flatMap[Int](s => null))
    show("filter")(s"${Optional.of(3).filter(_ > 2)} ${Optional.of(1).filter(_ > 2)}")
    show("or")(Optional.empty[String]().or(() => Optional.of("b")))
    show("orElse")(s"${Optional.empty[String]().orElse("c")} ${Optional.of("a").orElseGet(() => { note("got"); "d" })}")
    show("ifPresent")({ Optional.of("a").ifPresent(x => note("saw " + x)); Optional.empty[String]().ifPresent(null); "done" })
    show("ifPresentOrElse")({ Optional.empty[String]().ifPresentOrElse(x => note(x), () => note("none")); "done" })
    show("stream")(s"${Optional.of("a").stream().count()} ${Optional.empty[String]().stream().count()}")
    show("equals")(Optional.of("a") == Optional.of("a") && Optional.empty[String]() == Optional.ofNullable(null))
    show("hashCode")(s"${Optional.of(7).hashCode()} ${Optional.empty().hashCode()}")
    show("OptionalInt")(s"${OptionalInt.of(3)} ${OptionalInt.empty()} ${OptionalInt.of(3).getAsInt()}")
    show("OptionalInt empty get")(OptionalInt.empty().getAsInt())
    show("OptionalInt orElse")(s"${OptionalInt.empty().orElse(5)} ${OptionalInt.empty().orElseGet(() => 6)}")
    show("OptionalInt equals")(OptionalInt.of(3) == OptionalInt.of(3) && OptionalInt.empty() != OptionalInt.of(0))
    show("OptionalLong")(s"${OptionalLong.of(4L)} ${OptionalLong.empty()} ${OptionalLong.of(4L).getAsLong()}")
    show("OptionalDouble")(s"${OptionalDouble.of(2.5)} ${OptionalDouble.empty()}")
    show("OptionalDouble NaN equals")(OptionalDouble.of(Double.NaN) == OptionalDouble.of(Double.NaN))
    show("OptionalInt ifPresent")({ OptionalInt.of(9).ifPresent((v: Int) => note(v)); "done" })

  def collectors(): Unit =
    show("toList")(Stream.of("b", "a").collect(Collectors.toList[String]()))
    show("toList mutable")({ val l = Stream.of("b").collect(Collectors.toList[String]()); l.add("c"); l })
    show("toSet")(Stream.of(1, 2, 1, 3).collect(Collectors.toSet[Int]()))
    show("joining")(Stream.of("a", "b", "c").collect(Collectors.joining()))
    show("joining delimiter")(Stream.of("a", "b", "c").collect(Collectors.joining(", ")))
    show("joining all")(Stream.of("a", "b").collect(Collectors.joining(", ", "[", "]")))
    show("joining none")(Stream.empty[String]().collect(Collectors.joining(", ", "<", ">")))
    show("joining null element")(Stream.of("a", null).collect(Collectors.joining("-")))
    show("joining null delimiter")(Stream.of("a").collect(Collectors.joining(null, "", "")))
    show("counting")(Stream.of("a", "b", "c").collect(Collectors.counting[String]()))
    show("toMap")(Stream.of("a", "b", "c").collect(Collectors.toMap[String, String, String](s => s, s => s + s)))
    show("toMap duplicate")(Stream.of("a", "b", "a").collect(Collectors.toMap[String, String, Int](s => s, s => s.length)))
    show("toMap null value")(Stream.of("a").collect(Collectors.toMap[String, String, String](s => s, s => null)))
    show("toMap merge")(Stream.of("a", "b", "a").collect(Collectors.toMap[String, String, Int](s => s, s => 1, (x, y) => x + y)))
    show("toMap merge to null")(Stream.of("a", "b", "a").collect(Collectors.toMap[String, String, String](s => s, s => s, (x, y) => null)))
    show("toMap factory")(Stream.of("b", "a", "b").collect(Collectors.toMap[String, String, Int, java.util.TreeMap[String, Int]](s => s, s => 1, (x, y) => x + y, () => new java.util.TreeMap[String, Int]())))
    show("groupingBy")(Stream.of("a", "bb", "cc", "d").collect(Collectors.groupingBy[String, Int](s => s.length)))
    show("groupingBy counting")(Stream.of("a", "bb", "cc", "d", "e").collect(Collectors.groupingBy[String, Int, Any, java.lang.Long](s => s.length, Collectors.counting[String]().asInstanceOf[Collector[String, Any, java.lang.Long]])))
    show("groupingBy factory")(Stream.of("bb", "a", "cc").collect(Collectors.groupingBy[String, String, java.lang.Long, Any, java.util.TreeMap[String, java.lang.Long]](s => s.substring(0, 1), () => new java.util.TreeMap[String, java.lang.Long](), Collectors.counting[String]().asInstanceOf[Collector[String, Any, java.lang.Long]])))
    show("groupingBy null key")(Stream.of("a").collect(Collectors.groupingBy[String, String](s => null)))
    show("characteristics")(s"${Collectors.toList[String]().characteristics()} ${Collectors.toSet[String]().characteristics()} ${Collectors.joining().characteristics()}")
    val ofIdentity = Collector.of[String, ArrayList[String]](() => new ArrayList[String](), (l, s) => { l.add(s); () }, (a, b) => { a.addAll(b); a })
    show("Collector.of")(s"${Stream.of("x", "y").collect(ofIdentity)} ${ofIdentity.characteristics()}")
    val ofFinished = Collector.of[String, java.lang.StringBuilder, String](() => new java.lang.StringBuilder(), (b, s) => { b.append(s); () }, (a, b) => a, b => "<" + b + ">", Collector.Characteristics.UNORDERED)
    show("Collector.of finisher")(s"${Stream.of("x", "y").collect(ofFinished)} ${ofFinished.characteristics()}")
    // A collector of one's own: the stream asks for its functions first, its characteristics last.
    val own = new Collector[String, ArrayList[String], Int]:
      def supplier(): Supplier[ArrayList[String]] = { note("supplier"); () => { note("new"); new ArrayList[String]() } }
      def accumulator(): BiConsumer[ArrayList[String], String] = { note("accumulator"); (l, s) => { note(s); l.add(s); () } }
      def combiner(): BinaryOperator[ArrayList[String]] = { note("combiner"); (a, b) => a }
      def finisher(): Function[ArrayList[String], Int] = { note("finisher"); l => l.size() }
      def characteristics(): java.util.Set[Collector.Characteristics] = { note("characteristics"); java.util.Collections.emptySet() }
    show("own collector")(Stream.of("p", "q").collect(own))
    show("Characteristics")(s"${Collector.Characteristics.valueOf("CONCURRENT")} ${Collector.Characteristics.IDENTITY_FINISH.ordinal()}")
    // Whether the collector is unordered is asked once the stream is taken, before its source.
    show("characteristics asked")(Stream.of("p").collect(new Counted(new AskedTraits(Collector.Characteristics.IDENTITY_FINISH))))
    show("characteristics asked, finished")(Stream.of("p").collect(new Counted(new AskedTraits(Collector.Characteristics.UNORDERED))))
    show("null characteristics")(try Stream.of("p").collect(new Counted(null)) catch case e: NullPointerException => "NullPointerException")

  def members(): Unit =
    show("filter map")(Stream.of(1, 2, 3, 4).filter(_ % 2 == 0).map[String](x => "n" + x).toList())
    show("flatMap")(Stream.of("ab", "c").flatMap[Char](s => Stream.of(s.toCharArray.toSeq*)).toList())
    show("flatMapToInt")(Stream.of("aa", "b").flatMapToInt(s => IntStream.of(s.length, 10)).sum())
    show("flatMapToLong")(Stream.of(1, 2).flatMapToLong(x => LongStream.of(x.toLong * 3000000000L)).sum())
    show("flatMapToDouble")(Stream.of(1, 2).flatMapToDouble(x => DoubleStream.of(x + 0.25)).sum())
    show("mapMulti")(Stream.of(1, 2).mapMulti[String]((x, out) => { out.accept("a" + x); out.accept("b" + x) }).toList())
    show("mapMultiToInt")(Stream.of(1, 2).mapMultiToInt((x, out) => { out.accept(x); out.accept(x * 10) }).sum())
    show("mapMultiToLong")(Stream.of(1).mapMultiToLong((x, out) => out.accept(7L)).sum())
    show("mapMultiToDouble")(Stream.of(1).mapMultiToDouble((x, out) => out.accept(0.5)).sum())
    show("distinct")(Stream.of(3, 1, 3, 2, 1).distinct().toList())
    show("distinct null")(Stream.of[String]("a", null, "a", null).distinct().toList())
    show("sorted")(Stream.of("b", "c", "a").sorted().toList())
    show("sorted comparator")(Stream.of("bb", "a", "cc", "d").sorted((x, y) => x.length - y.length).toList())
    show("peek")(Stream.of(1, 2).peek(x => note("p" + x)).map[Int](x => x * 10).toList())
    show("limit skip")(Stream.of(1, 2, 3, 4, 5).skip(1).limit(3).toList())
    show("takeWhile dropWhile")(s"${Stream.of(1, 2, 5, 1).takeWhile(_ < 3).toList()} ${Stream.of(1, 2, 5, 1).dropWhile(_ < 3).toList()}")
    show("forEach")({ Stream.of(1, 2).forEach(x => note(x)); "done" })
    show("forEachOrdered")({ Stream.of(1, 2).map[Int](x => x + 1).forEachOrdered(x => note(x)); "done" })
    show("toArray")(Stream.of(1, 2).toArray().mkString(","))
    show("toArray generator")(Stream.of("a", "b").toArray(n => new Array[String](n)).mkString(","))
    show("reduce identity")(Stream.of(1, 2, 3).reduce(10, (a, b) => a + b))
    show("reduce")(s"${Stream.of(1, 2, 3).reduce((a, b) => a * b)} ${Stream.empty[Int]().reduce((a, b) => a * b)}")
    show("reduce three")(Stream.of("aa", "b").reduce[Int](0, (n, s) => n + s.length, (a, b) => a + b))
    show("collect three")(Stream.of("x", "y").collect[ArrayList[String]](() => new ArrayList[String](), (a, s) => { a.add(s); () }, (a, b) => { a.addAll(b); () }))
    show("toList")(Stream.of("a", null).toList())
    show("min max")(s"${Stream.of("bb", "a", "ccc").min((x, y) => x.length - y.length)} ${Stream.of("bb", "a", "ccc").max((x, y) => x.length - y.length)}")
    show("min max ties")(s"${Stream.of("a", "b").min((x, y) => 0)} ${Stream.of("a", "b").max((x, y) => 0)}")
    show("count")(Stream.of(1, 2, 3).filter(_ > 1).count())
    show("matches")(s"${Stream.of(1, 2).anyMatch(_ > 1)} ${Stream.of(1, 2).allMatch(_ > 1)} ${Stream.of(1, 2).noneMatch(_ > 2)}")
    show("matches empty")(s"${Stream.empty[Int]().anyMatch(_ => true)} ${Stream.empty[Int]().allMatch(_ => false)} ${Stream.empty[Int]().noneMatch(_ => true)}")
    show("findFirst findAny")(s"${Stream.of(4, 5).findFirst()} ${Stream.of(6).findAny()} ${Stream.empty[Int]().findFirst()}")
    show("iterator")({ val it = Stream.of(1, 2).iterator(); var s = ""; while it.hasNext do s += it.next(); s })
    show("spliterator")({
      val sp = Stream.of("a", "b", "c").spliterator()
      val before = s"${sp.estimateSize()} ${sp.getExactSizeIfKnown()} ${sp.hasCharacteristics(java.util.Spliterator.SIZED)}"
      sp.tryAdvance(x => note(x))
      s"$before ${sp.estimateSize()}"
    })
    show("unsized spliterator")(Stream.iterate(1, x => x + 1).spliterator().getExactSizeIfKnown())
    show("parallel")({ val s = Stream.of(1, 2).parallel(); val p = s.isParallel(); s"$p ${s.sequential().isParallel()} ${s.map[Int](x => x).count()}" })
    show("unordered")(Stream.of(2, 1).unordered().toList())
    show("Function.identity")(Stream.of("a", "b").map(Function.identity[String]()).toList())
    show("BinaryOperator.minBy")(Stream.of(3, 1, 2).reduce(BinaryOperator.minBy[Int]((a, b) => a - b)))
    show("sorted twice sorts once")({ comparisons = 0; val l = Stream.of(new Item(2), new Item(1)).sorted().sorted().toList(); s"$l $comparisons" })
    show("sorted then comparator sorts again")({ comparisons = 0; val l = Stream.of(new Item(2), new Item(1)).sorted().sorted((a, b) => b.compareTo(a)).toList(); s"$l $comparisons" })
    show("sorted then distinct")({ comparisons = 0; Stream.of(new Item(1), new Item(2)).sorted().distinct().count() })
    show("stage characteristics")({
      import java.util.Spliterator.{DISTINCT, ORDERED, SIZED, SORTED}
      def flags(sp: java.util.Spliterator[?]): String =
        Seq(ORDERED -> "ordered", SORTED -> "sorted", DISTINCT -> "distinct", SIZED -> "sized").collect { case (f, n) if sp.hasCharacteristics(f) => n }.mkString("+")
      val sorted = Stream.of("b", "a").sorted().spliterator()
      s"${flags(sorted)} ${sorted.getComparator()} ${flags(Stream.of("a").distinct().spliterator())} ${flags(Stream.of("a").map[String](x => x).spliterator())} ${flags(Stream.of("b", "a").sorted((x, y) => x.compareTo(y)).spliterator())}"
    })
    show("unsorted getComparator")(Stream.of("a").map[String](x => x).spliterator().getComparator())
    // A source sorted in natural order has no comparator; one that is not sorted has none to ask.
    show("sorted source")({
      val sp = java.util.Spliterators.spliterator(sortedList(), java.util.Spliterator.SORTED | java.util.Spliterator.ORDERED)
      s"${sp.getComparator()} ${java.util.stream.StreamSupport.stream(sp, false).count()}"
    })
    show("unsorted source comparator")(java.util.Spliterators.spliterator(sortedList(), java.util.Spliterator.ORDERED).getComparator())
    show("supplied source comparator")(java.util.stream.StreamSupport.stream[String](() => java.util.Spliterators.spliterator(sortedList(), java.util.Spliterator.SORTED | java.util.Spliterator.ORDERED), java.util.Spliterator.SORTED | java.util.Spliterator.ORDERED, false).spliterator().getComparator())
    show("concatenation comparator")({
      val sorted = java.util.stream.StreamSupport.stream(java.util.Spliterators.spliterator(sortedList(), java.util.Spliterator.SORTED | java.util.Spliterator.ORDERED), false)
      val c = Stream.concat(Stream.of("x"), sorted).spliterator()
      val before = try String.valueOf(c.getComparator()) catch case e: IllegalStateException => "IllegalStateException"
      c.tryAdvance(x => note(x))
      c.tryAdvance(x => note(x))
      s"$before ${c.hasCharacteristics(java.util.Spliterator.SORTED)} ${c.getComparator()}"
    })
    show("list stream is ordered")({
      val list = new ArrayList[String]()
      list.add("a")
      val stream = list.stream()
      val unordered = stream.unordered()
      s"${stream eq unordered} ${list.spliterator().hasCharacteristics(java.util.Spliterator.ORDERED)} ${unordered.count()}"
    })
    show("list stream used once")({ val list = new ArrayList[String](); val stream = list.stream(); stream.unordered(); stream.count() })
    show("set stream is distinct")({ val set = new java.util.HashSet[Int](); set.add(1); set.spliterator().hasCharacteristics(java.util.Spliterator.DISTINCT) })
    show("Collector.of null")(Collector.of[String, ArrayList[String]](() => new ArrayList[String](), (l, x) => { l.add(x); () }, (a, b) => a, null.asInstanceOf[Collector.Characteristics]))
    show("array spliced in")({
      val values = Array("before", "second")
      val s = Stream.of[String](values*)
      values(0) = "after"
      val ints = Array(1, 2)
      val is = IntStream.of(ints*)
      ints(0) = 9
      s"${s.toList()} ${is.sum()}"
    })

  def primitives(): Unit =
    show("int sum")(Stream.of("a", "bb", "ccc").mapToInt(_.length).sum())
    show("int overflow")(Stream.of(Int.MaxValue, 1).mapToInt(x => x).sum())
    show("int min max")(s"${Stream.of(3, 1, 2).mapToInt(x => x).min()} ${Stream.of(3, 1, 2).mapToInt(x => x).max()} ${Stream.empty[Int]().mapToInt(x => x).max()}")
    show("int average")(s"${Stream.of(1, 2).mapToInt(x => x).average()} ${Stream.of(Int.MaxValue, Int.MaxValue).mapToInt(x => x).average().getAsDouble() == Int.MaxValue.toDouble} ${Stream.empty[Int]().mapToInt(x => x).average()}")
    show("int count")(Stream.of(1, 2).mapToInt(x => x).count())
    show("int toArray")(Stream.of(3, 4).mapToInt(x => x).toArray().mkString(","))
    show("int boxed")(Stream.of(3, 4).mapToInt(x => x * 2).boxed().toList())
    show("int iterator")({ val it = IntStream.of(5, 6).iterator(); s"${it.nextInt()} ${it.next()} ${it.hasNext}" })
    show("primitive spliterators")({
      val ints: java.util.Spliterator.OfInt = IntStream.of(1, 2).spliterator()
      val consumer: IntConsumer = v => note(v)
      ints.tryAdvance(consumer)
      val longs: java.util.Spliterator.OfLong = LongStream.of(1L, 2L, 3L).spliterator()
      val doubles: java.util.Spliterator.OfDouble = DoubleStream.of(1.5).spliterator()
      s"${ints.estimateSize()} ${longs.estimateSize()} ${doubles.estimateSize()}"
    })
    show("int forEach")({ IntStream.of(1, 2).forEach((v: Int) => note(v)); "done" })
    show("int empty")(s"${IntStream.empty().count()} ${IntStream.of(8).sum()}")
    show("empty has no order")({ val e = IntStream.empty(); val f = Stream.empty[Int](); s"${e.unordered() eq e} ${e.count()} ${f.unordered() eq f} ${f.count()}" })
    show("optional streams")(s"${OptionalInt.of(3).stream().sum()} ${OptionalInt.empty().stream().count()} ${OptionalLong.of(4L).stream().sum()} ${OptionalDouble.empty().stream().count()}")
    show("long")(s"${Stream.of(1, 2).mapToLong(x => x * 4000000000L).sum()} ${LongStream.of(5L, 2L).min()} ${LongStream.of(5L, 2L).max()} ${LongStream.of(1L, 2L).average()}")
    show("long toArray boxed")(s"${LongStream.of(1L, 2L).toArray().mkString(",")} ${LongStream.of(3L).boxed().toList()}")
    show("double")(s"${Stream.of(1, 2).mapToDouble(x => x + 0.25).sum()} ${DoubleStream.of(2.5, -1.5).min()} ${DoubleStream.of(2.5, -1.5).max()} ${DoubleStream.of(1.5, 2.0).average()}")
    show("double compensated")({
      val tenths = Stream.generate[Double](() => 0.1).limit(10).mapToDouble(x => x)
      val naive = Stream.generate[Double](() => 0.1).limit(10).reduce(0.0, (a, b) => a + b)
      s"${tenths.sum() == 1.0} ${naive == 1.0}"
    })
    show("double NaN infinity")(s"${DoubleStream.of(1.5, Double.NaN).max().getAsDouble().isNaN} ${DoubleStream.of(Double.PositiveInfinity, Double.PositiveInfinity).sum() == Double.PositiveInfinity} ${java.lang.Double.compare(DoubleStream.of(-0.0, 0.0).min().getAsDouble(), 0.0)}")
    show("double toArray boxed")(s"${DoubleStream.of(1.5, 2.5).toArray().mkString(",")} ${DoubleStream.of(0.5).boxed().toList()}")

  def laziness(): Unit =
    show("nothing before the terminal")({ Stream.of(1, 2).peek(x => note("p" + x)).map[Int](x => { note("m" + x); x }); "linked" })
    show("one element at a time")(Stream.of(1, 2).peek(x => note("a" + x)).filter(x => { note("f" + x); true }).peek(x => note("b" + x)).toList())
    show("sized count runs nothing")(Stream.of(1, 2, 3).peek(x => note("p" + x)).map[Int](x => { note("m" + x); x }).count())
    show("sized count limit skip")(s"${Stream.of(1, 2, 3, 4).peek(x => note(x)).skip(1).limit(2).count()} ${Stream.of(1, 2).sorted().peek(x => note(x)).count()}")
    show("unsized count runs")(Stream.of(1, 2, 3).peek(x => note("p" + x)).filter(_ > 1).count())
    show("concat sized count")(Stream.concat(Stream.of(1), Stream.of(2, 3).limit(1)).peek(x => note(x)).count())
    show("limit stops the source")(Stream.of(1, 2, 3, 4).peek(x => note("p" + x)).limit(2).toList())
    show("iterate limit")(Stream.iterate(1, x => { note("f" + x); x * 2 }).limit(4).toList())
    show("generate filter limit")({ var n = 0; val l = Stream.generate[Int](() => { n += 1; n }).filter(_ % 2 == 0).limit(2).toList(); s"$l after $n" })
    show("sorted then findFirst")(Stream.of(3, 1, 2).peek(x => note("in" + x)).sorted().peek(x => note("out" + x)).findFirst())
    show("sorted then limit")(Stream.of(3, 1, 2, 0).sorted().peek(x => note(x)).limit(2).toList())
    show("anyMatch stops")(Stream.of(1, 2, 3).peek(x => note(x)).anyMatch(_ == 2))
    show("allMatch stops")(Stream.of(1, 2, 3).peek(x => note(x)).allMatch(_ < 2))
    show("takeWhile stops")(Stream.iterate(1, x => x + 1).peek(x => note(x)).takeWhile(_ < 3).toList())
    show("dropWhile")(Stream.of(1, 2, 3, 1).peek(x => note(x)).dropWhile(_ < 2).toList())
    show("flatMap closes each")(Stream.of(1, 2).flatMap[Int](x => Stream.of(x, x * 10).onClose(() => note("closed" + x))).toList())
    show("flatMap stops early")(Stream.of(1, 2, 3).flatMap[Int](x => Stream.of(x, x * 10).onClose(() => note("closed" + x))).peek(x => note("p" + x)).anyMatch(_ == 10))
    show("flatMap limit")(Stream.of(1, 2).flatMap[Int](x => Stream.of(x, x + 1, x + 2).peek(y => note("in" + y))).limit(2).toList())
    show("flatMap null")(Stream.of(1, 2).flatMap[Int](x => if x == 1 then null else Stream.of(x)).toList())
    show("mapMulti limit")(Stream.of(1, 2).mapMulti[Int]((x, out) => { note("m" + x); out.accept(x); out.accept(x) }).limit(1).toList())
    show("iterator on demand")({
      val it = Stream.of(1, 2, 3).peek(x => note("p" + x)).map[Int](x => x * 2).iterator()
      note("made")
      val h = it.hasNext
      note("again")
      it.hasNext
      s"$h ${it.next()}"
    })
    show("iterator of sorted")({ val it = Stream.of(3, 1, 2).peek(x => note(x)).sorted().iterator(); note("made"); it.next() })
    show("toArray generator first when sized")(Stream.of("a", "b").peek(x => note(x)).toArray(n => { note("array" + n); new Array[String](n) }).mkString(","))
    show("toArray generator last when unsized")(Stream.of("a", "b").filter(_ => true).peek(x => note(x)).toArray(n => { note("array" + n); new Array[String](n) }).mkString(","))
    show("toArray generator too short")(Stream.of("a", "b").toArray(n => new Array[String](1)).length)
    show("toArray unsized too short")(Stream.of("a", "b").filter(_ => true).toArray(n => new Array[String](1)).length)
    show("iterate three")(Stream.iterate[Int](1, x => { note("has" + x); x < 4 }, x => { note("next" + x); x + 1 }).toList())
    show("iterate three findFirst")(Stream.iterate[Int](1, x => { note("has" + x); x < 4 }, x => { note("next" + x); x + 1 }).findFirst())

  def lifecycle(): Unit =
    val s = Stream.of(1, 2)
    show("map null")(s.map[Int](null))
    show("after a refused map")(s.map[Int](x => x).toList())
    show("second use")(s.map[Int](x => x))
    show("terminal twice")({ val t = Stream.of(1); t.count(); t.count() })
    show("filter null")(Stream.of(1).filter(null))
    show("peek null")(Stream.of(1).peek(null))
    show("sorted null links")({ val t = Stream.of(2, 1); try t.sorted(null) catch case e: NullPointerException => note("npe"); t.count() })
    show("reduce null")(Stream.of(1).reduce(null))
    show("min null")(Stream.of(1).min(null))
    show("findFirst of null")(Stream.of[String](null, "a").findFirst())
    show("reduce to null")(Stream.of[String](null, null).reduce((a, b) => a))
    val h = Stream.of(1)
    show("forEach null on the source")(h.forEach(null))
    show("after it")(h.count())
    val d = Stream.of(1).map[Int](x => x)
    show("forEach null on a stage")(d.forEach(null))
    show("after that")(d.count())
    val z = Stream.of(1, 2)
    show("skip 0 is the stream")(z.skip(0) eq z)
    show("and stays usable")(z.skip(0).count())
    show("limit negative")(Stream.of(1).limit(-1))
    show("skip negative")(Stream.of(1).skip(-2))
    val src = Stream.of(1, 2)
    val der = src.map[Int](x => x)
    src.close()
    show("stage after its source closed")(der.toList())
    val c2 = Stream.of(1, 2)
    val d2 = c2.map[Int](x => x)
    d2.close()
    show("source after its stage closed")(c2.toList())
    show("closed stage")(d2.toList())
    show("close handlers")({
      val root = Stream.of(1).onClose(() => note("root"))
      val child = root.map[Int](x => x).onClose(() => note("child"))
      child.close()
      child.close()
      root.close()
      "closed"
    })
    show("onClose after use")({ val t = Stream.of(1); t.count(); t.onClose(() => ()) })
    show("onClose null")(Stream.of(1).onClose(null))
    show("close exceptions")({
      val t = Stream.of(1)
        .onClose(() => { note("first"); throw new IllegalStateException("one") })
        .onClose(() => { note("second"); throw new IllegalArgumentException("two") })
        .onClose(() => note("third"))
      try { t.close(); "no exception" }
      catch case e: IllegalStateException => e.getMessage
    })
    show("toList closes nothing")({ val t = Stream.of(1).onClose(() => note("closed")); t.toList(); "listed" })
    show("toList unmodifiable")(Stream.of(1).toList().add(2))
    show("toList bounds")(Stream.of(1).toList().get(3))
    show("concat")({
      val a = Stream.of(1, 2).onClose(() => note("a"))
      val b = Stream.of(3).onClose(() => note("b"))
      val c = Stream.concat(a, b)
      val l = c.toList()
      c.close()
      l
    })
    show("concat takes both")({ val a = Stream.of(1); Stream.concat(a, Stream.of(2)); a.count() })
    show("concat used")({ val a = Stream.of(1); a.count(); Stream.concat(a, Stream.of(2)) })
    show("builder")({ val b = Stream.builder[String](); b.add("x").add("y"); b.accept("z"); b.build().toList() })
    show("builder after build")({ val b = Stream.builder[String](); b.build(); b.add("x") })

  def sources(): Unit =
    show("empty")(s"${Stream.empty[String]().count()} ${Stream.empty[String]().toList()}")
    show("of one")(Stream.of("a").toList())
    show("ofNullable")(s"${Stream.ofNullable[String](null).count()} ${Stream.ofNullable("a").toList()}")
    show("generate limit")(Stream.generate[String](() => "g").limit(3).toList())
    val list = new ArrayList[String]()
    list.add("abc")
    list.add("de")
    show("ArrayList stream")(s"${list.stream().mapToInt(_.length).sum()} ${list.stream().map[String](_.toUpperCase).toList()}")
    val late = list.stream()
    list.add("f")
    show("ArrayList stream binds late")(late.count())
    val set = new java.util.HashSet[Int]()
    set.add(1)
    set.add(2)
    show("HashSet stream")(set.stream().map[Int](_ * 10).toList())
