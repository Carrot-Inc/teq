// `java.util.stream` as the JDK's sequential pipeline runs it (`AbstractPipeline`): a stream is a
// stage over a source or over the stage before it, and nothing runs until a terminal operation,
// which pushes the source's elements one at a time through each stage's sink to its own. An
// intermediate operation checks its function first, then takes the stage it builds on: a stage is
// used once, by the next stage or by a terminal operation, and a second use, or a use after
// `close`, is an `IllegalStateException`. A pipeline with a stage or a terminal operation that
// stops early (`limit`, `takeWhile`, `findFirst`, the matches) asks before each element whether to
// go on; one whose size its source knows and no stage changes but by slicing is counted without a
// traversal. `close` runs the source's handlers once, in the order they were given, the first
// exception one throws thrown after the others ran; neither `toList` nor the iterator closes the
// stream. `parallel` is recorded and runs sequentially. On the JVM the classes are the JDK's.
package java.util.stream:

  @jvmClass("java/util/stream/BaseStream")
  trait BaseStream[T, S <: BaseStream[T, S]] extends java.lang.AutoCloseable:
    def iterator(): java.util.Iterator[T]
    def spliterator(): java.util.Spliterator[T]
    def isParallel(): Boolean
    def sequential(): S
    def parallel(): S
    def unordered(): S
    def onClose(closeHandler: Runnable): S
    def close(): Unit

  @jvmClass("java/util/stream/Stream")
  trait Stream[T] extends BaseStream[T, Stream[T]]:
    def filter(predicate: java.util.function.Predicate[? >: T]): Stream[T]
    def map[R](mapper: java.util.function.Function[? >: T, ? <: R]): Stream[R]
    def mapToInt(mapper: java.util.function.ToIntFunction[? >: T]): IntStream
    def mapToLong(mapper: java.util.function.ToLongFunction[? >: T]): LongStream
    def mapToDouble(mapper: java.util.function.ToDoubleFunction[? >: T]): DoubleStream
    def flatMap[R](mapper: java.util.function.Function[? >: T, ? <: Stream[? <: R]]): Stream[R]
    def flatMapToInt(mapper: java.util.function.Function[? >: T, ? <: IntStream]): IntStream
    def flatMapToLong(mapper: java.util.function.Function[? >: T, ? <: LongStream]): LongStream
    def flatMapToDouble(mapper: java.util.function.Function[? >: T, ? <: DoubleStream]): DoubleStream
    def mapMulti[R](mapper: java.util.function.BiConsumer[? >: T, ? >: java.util.function.Consumer[R]]): Stream[R]
    def mapMultiToInt(mapper: java.util.function.BiConsumer[? >: T, ? >: java.util.function.IntConsumer]): IntStream
    def mapMultiToLong(mapper: java.util.function.BiConsumer[? >: T, ? >: java.util.function.LongConsumer]): LongStream
    def mapMultiToDouble(mapper: java.util.function.BiConsumer[? >: T, ? >: java.util.function.DoubleConsumer]): DoubleStream
    def distinct(): Stream[T]
    def sorted(): Stream[T]
    def sorted(comparator: java.util.Comparator[? >: T]): Stream[T]
    def peek(action: java.util.function.Consumer[? >: T]): Stream[T]
    def limit(maxSize: Long): Stream[T]
    def skip(n: Long): Stream[T]
    def takeWhile(predicate: java.util.function.Predicate[? >: T]): Stream[T]
    def dropWhile(predicate: java.util.function.Predicate[? >: T]): Stream[T]
    def forEach(action: java.util.function.Consumer[? >: T]): Unit
    def forEachOrdered(action: java.util.function.Consumer[? >: T]): Unit
    def toArray(): Array[AnyRef]
    def toArray[A](generator: java.util.function.IntFunction[Array[A & AnyRef]]): Array[A & AnyRef]
    def reduce(identity: T, accumulator: java.util.function.BinaryOperator[T]): T
    def reduce(accumulator: java.util.function.BinaryOperator[T]): java.util.Optional[T]
    def reduce[U](identity: U, accumulator: java.util.function.BiFunction[U, ? >: T, U], combiner: java.util.function.BinaryOperator[U]): U
    def collect[R](supplier: java.util.function.Supplier[R], accumulator: java.util.function.BiConsumer[R, ? >: T], combiner: java.util.function.BiConsumer[R, R]): R
    def collect[R, A](collector: Collector[? >: T, A, R]): R
    def toList(): java.util.List[T]
    def min(comparator: java.util.Comparator[? >: T]): java.util.Optional[T]
    def max(comparator: java.util.Comparator[? >: T]): java.util.Optional[T]
    def count(): Long
    def anyMatch(predicate: java.util.function.Predicate[? >: T]): Boolean
    def allMatch(predicate: java.util.function.Predicate[? >: T]): Boolean
    def noneMatch(predicate: java.util.function.Predicate[? >: T]): Boolean
    def findFirst(): java.util.Optional[T]
    def findAny(): java.util.Optional[T]

  @jvmClass("java/util/stream/Stream")
  object Stream:
    @jvmClass("java/util/stream/Stream$Builder")
    trait Builder[T] extends java.util.function.Consumer[T]:
      def accept(t: T): Unit
      def add(t: T): Builder[T] =
        accept(t)
        this
      def build(): Stream[T]

    def builder[T](): Builder[T] = new StreamBuilder[T]
    def empty[T](): Stream[T] = StreamSupport.stream(java.util.Spliterators.emptySpliterator[T](), false)
    def of[T](t: T): Stream[T] = Pipes.refs[T](Array[Any](t))
    def ofNullable[T](t: T): Stream[T] = if t == null then empty[T]() else of[T](t)
    def of[T](values: T*): Stream[T] = Pipes.refs[T](Pipes.arrayOf(values))
    def iterate[T](seed: T, f: java.util.function.UnaryOperator[T]): Stream[T] =
      if f == null then throw new NullPointerException()
      StreamSupport.stream(new Iterating[T](seed, f), false)
    def iterate[T](seed: T, hasNext: java.util.function.Predicate[? >: T], next: java.util.function.UnaryOperator[T]): Stream[T] =
      if next == null then throw new NullPointerException()
      if hasNext == null then throw new NullPointerException()
      StreamSupport.stream(new IteratingWhile[T](seed, hasNext.asInstanceOf[java.util.function.Predicate[T]], next), false)
    def generate[T](s: java.util.function.Supplier[? <: T]): Stream[T] =
      if s == null then throw new NullPointerException()
      StreamSupport.stream(new Generating[T](s.asInstanceOf[java.util.function.Supplier[T]]), false)
    // Both streams taken now, as `spliterator` takes them; closing the result closes both.
    def concat[T](a: Stream[? <: T], b: Stream[? <: T]): Stream[T] =
      if a == null then throw new NullPointerException()
      if b == null then throw new NullPointerException()
      val split = new Concatenation[T](a.spliterator().asInstanceOf[java.util.Spliterator[T]], b.spliterator().asInstanceOf[java.util.Spliterator[T]])
      StreamSupport.stream(split, a.isParallel() || b.isParallel()).onClose(Pipes.closeBoth(a, b))

  @jvmClass("java/util/stream/IntStream")
  trait IntStream extends BaseStream[java.lang.Integer, IntStream]:
    override def iterator(): java.util.PrimitiveIterator.OfInt
    override def spliterator(): java.util.Spliterator.OfInt
    def forEach(action: java.util.function.IntConsumer): Unit
    def toArray(): Array[Int]
    def sum(): Int
    def min(): java.util.OptionalInt
    def max(): java.util.OptionalInt
    def count(): Long
    def average(): java.util.OptionalDouble
    def boxed(): Stream[java.lang.Integer]

  @jvmClass("java/util/stream/IntStream")
  object IntStream:
    def empty(): IntStream = Pipes.ints(new Array[Any](0), Pipes.EMPTY)
    def of(t: Int): IntStream = Pipes.ints(Array[Any](t))
    def of(values: Int*): IntStream = Pipes.ints(Pipes.arrayOf(values))

  @jvmClass("java/util/stream/LongStream")
  trait LongStream extends BaseStream[java.lang.Long, LongStream]:
    override def iterator(): java.util.PrimitiveIterator.OfLong
    override def spliterator(): java.util.Spliterator.OfLong
    def forEach(action: java.util.function.LongConsumer): Unit
    def toArray(): Array[Long]
    def sum(): Long
    def min(): java.util.OptionalLong
    def max(): java.util.OptionalLong
    def count(): Long
    def average(): java.util.OptionalDouble
    def boxed(): Stream[java.lang.Long]

  @jvmClass("java/util/stream/LongStream")
  object LongStream:
    def empty(): LongStream = Pipes.longs(new Array[Any](0), Pipes.EMPTY)
    def of(t: Long): LongStream = Pipes.longs(Array[Any](t))
    def of(values: Long*): LongStream = Pipes.longs(Pipes.arrayOf(values))

  @jvmClass("java/util/stream/DoubleStream")
  trait DoubleStream extends BaseStream[java.lang.Double, DoubleStream]:
    override def iterator(): java.util.PrimitiveIterator.OfDouble
    override def spliterator(): java.util.Spliterator.OfDouble
    def forEach(action: java.util.function.DoubleConsumer): Unit
    def toArray(): Array[Double]
    def sum(): Double
    def min(): java.util.OptionalDouble
    def max(): java.util.OptionalDouble
    def count(): Long
    def average(): java.util.OptionalDouble
    def boxed(): Stream[java.lang.Double]

  @jvmClass("java/util/stream/DoubleStream")
  object DoubleStream:
    def empty(): DoubleStream = Pipes.doubles(new Array[Any](0), Pipes.EMPTY)
    def of(t: Double): DoubleStream = Pipes.doubles(Array[Any](t))
    def of(values: Double*): DoubleStream = Pipes.doubles(Pipes.arrayOf(values))

  @jvmClass("java/util/stream/StreamSupport")
  object StreamSupport:
    def stream[T](spliterator: java.util.Spliterator[T], parallel: Boolean): Stream[T] =
      if spliterator == null then throw new NullPointerException()
      val head = new RefPipe[T](null, null, Pipes.flagsOf(spliterator))
      head.source = spliterator.asInstanceOf[java.util.Spliterator[Any]]
      head.parallelFlag = parallel
      head
    // The source asked for at the terminal operation; its characteristics are told now.
    def stream[T](supplier: java.util.function.Supplier[? <: java.util.Spliterator[T]], characteristics: Int, parallel: Boolean): Stream[T] =
      if supplier == null then throw new NullPointerException()
      val head = new RefPipe[T](null, null, characteristics)
      head.supplier = supplier.asInstanceOf[java.util.function.Supplier[java.util.Spliterator[Any]]]
      head.parallelFlag = parallel
      head

  // ---- collectors ----

  @jvmClass("java/util/stream/Collector")
  trait Collector[T, A, R]:
    def supplier(): java.util.function.Supplier[A]
    def accumulator(): java.util.function.BiConsumer[A, T]
    def combiner(): java.util.function.BinaryOperator[A]
    def finisher(): java.util.function.Function[A, R]
    def characteristics(): java.util.Set[Collector.Characteristics]

  @jvmClass("java/util/stream/Collector")
  object Collector:
    @jvmClass("java/util/stream/Collector$Characteristics")
    final class Characteristics private[stream] (label: String, position: Int) extends java.lang.Enum[Characteristics]:
      override def name(): String = label
      override def ordinal(): Int = position
      override def toString: String = label
      override def compareTo(that: Characteristics): Int = position - that.ordinal()
      override def hashCode: Int = label.hashCode
      override def equals(that: Any): Boolean = that.asInstanceOf[AnyRef] eq this

    @jvmClass("java/util/stream/Collector$Characteristics")
    object Characteristics:
      val CONCURRENT: Characteristics = new Characteristics("CONCURRENT", 0)
      val UNORDERED: Characteristics = new Characteristics("UNORDERED", 1)
      val IDENTITY_FINISH: Characteristics = new Characteristics("IDENTITY_FINISH", 2)
      def values(): Array[Characteristics] = Array(CONCURRENT, UNORDERED, IDENTITY_FINISH)
      def valueOf(name: String): Characteristics = name match
        case "CONCURRENT" => CONCURRENT
        case "UNORDERED" => UNORDERED
        case "IDENTITY_FINISH" => IDENTITY_FINISH
        case null => throw new NullPointerException("Name is null")
        case _ => throw new IllegalArgumentException("No enum constant java.util.stream.Collector.Characteristics." + name)

    // Without characteristics the collector finishes by identity; with some, those and that.
    def of[T, R](supplier: java.util.function.Supplier[R], accumulator: java.util.function.BiConsumer[R, T], combiner: java.util.function.BinaryOperator[R], characteristics: Characteristics*): Collector[T, R, R] =
      if supplier == null || accumulator == null || combiner == null || characteristics == null then throw new NullPointerException()
      val cs = Collectors.traits(Characteristics.IDENTITY_FINISH +: characteristics)
      new CollectorImpl[T, R, R](supplier, accumulator, combiner, null, cs)
    def of[T, A, R](supplier: java.util.function.Supplier[A], accumulator: java.util.function.BiConsumer[A, T], combiner: java.util.function.BinaryOperator[A], finisher: java.util.function.Function[A, R], characteristics: Characteristics*): Collector[T, A, R] =
      if supplier == null || accumulator == null || combiner == null || finisher == null || characteristics == null then throw new NullPointerException()
      new CollectorImpl[T, A, R](supplier, accumulator, combiner, finisher, Collectors.traits(characteristics))

  // `Collectors.CollectorImpl`: one finishing by identity has no finisher of its own.
  private[java] final class CollectorImpl[T, A, R](
      supply: java.util.function.Supplier[A],
      accumulate: java.util.function.BiConsumer[A, T],
      combine: java.util.function.BinaryOperator[A],
      finish: java.util.function.Function[A, R],
      traits: java.util.Set[Collector.Characteristics]) extends Collector[T, A, R]:
    def supplier(): java.util.function.Supplier[A] = supply
    def accumulator(): java.util.function.BiConsumer[A, T] = accumulate
    def combiner(): java.util.function.BinaryOperator[A] = combine
    def finisher(): java.util.function.Function[A, R] =
      if finish != null then finish else (a: A) => a.asInstanceOf[R]
    def characteristics(): java.util.Set[Collector.Characteristics] = traits

  @jvmClass("java/util/stream/Collectors")
  object Collectors:
    // As an `EnumSet`'s: a null element refused, with the JVM's message of it.
    private[stream] def traits(cs: Seq[Collector.Characteristics]): java.util.Set[Collector.Characteristics] =
      if cs.exists(c => c == null) then throw new NullPointerException("Cannot invoke \"Object.getClass()\" because \"e\" is null")
      val out = new java.util.HashSet[Collector.Characteristics]()
      // In the order of their declaration, as an `EnumSet`'s.
      for c <- Collector.Characteristics.values() do
        if cs.exists(x => x eq c) then out.add(c)
      java.util.Collections.unmodifiableSet(out)
    private val ID = traits(Seq(Collector.Characteristics.IDENTITY_FINISH))
    private val UNORDERED_ID = traits(Seq(Collector.Characteristics.UNORDERED, Collector.Characteristics.IDENTITY_FINISH))
    private val NO_ID = traits(Seq())

    def toList[T](): Collector[T, ?, java.util.List[T]] =
      new CollectorImpl[T, java.util.List[T], java.util.List[T]](
        () => new java.util.ArrayList[T](),
        (l, t) => { l.add(t); () },
        (left, right) => { left.addAll(right); left },
        null, ID)
    def toSet[T](): Collector[T, ?, java.util.Set[T]] =
      new CollectorImpl[T, java.util.Set[T], java.util.Set[T]](
        () => new java.util.HashSet[T](),
        (s, t) => { s.add(t); () },
        (left, right) => if left.size() < right.size() then { right.addAll(left); right } else { left.addAll(right); left },
        null, UNORDERED_ID)
    def joining(): Collector[CharSequence, ?, String] =
      new CollectorImpl[CharSequence, java.lang.StringBuilder, String](
        () => new java.lang.StringBuilder(),
        (b, s) => { b.append(String.valueOf(s)); () },
        (b1, b2) => { b1.append(b2.toString); b1 },
        b => b.toString, NO_ID)
    def joining(delimiter: CharSequence): Collector[CharSequence, ?, String] = joining(delimiter, "", "")
    // The joiner is made, and its arguments checked, when the collection starts.
    def joining(delimiter: CharSequence, prefix: CharSequence, suffix: CharSequence): Collector[CharSequence, ?, String] =
      new CollectorImpl[CharSequence, java.util.StringJoiner, String](
        () => new java.util.StringJoiner(delimiter, prefix, suffix),
        (j, s) => { j.add(s); () },
        (j1, j2) => j1.merge(j2),
        j => j.toString, NO_ID)
    def counting[T](): Collector[T, ?, java.lang.Long] =
      new CollectorImpl[T, Array[Long], java.lang.Long](
        () => new Array[Long](1),
        (a, t) => a(0) = a(0) + 1L,
        (a, b) => { a(0) = a(0) + b(0); a },
        a => java.lang.Long.valueOf(a(0)), NO_ID)

    // A key mapped twice is an `IllegalStateException`, a null value a `NullPointerException`.
    def toMap[T, K, U](keyMapper: java.util.function.Function[? >: T, ? <: K], valueMapper: java.util.function.Function[? >: T, ? <: U]): Collector[T, ?, java.util.Map[K, U]] =
      val keyOf = keyMapper.asInstanceOf[java.util.function.Function[T, K]]
      val valueOf = valueMapper.asInstanceOf[java.util.function.Function[T, U]]
      new CollectorImpl[T, java.util.Map[K, U], java.util.Map[K, U]](
        () => new java.util.HashMap[K, U](),
        (map, element) => {
          val k = keyOf.apply(element)
          val v = valueOf.apply(element)
          if v == null then throw new NullPointerException()
          val u = map.get(k)
          if u != null then throw Collectors.duplicateKey(k, u, v)
          map.put(k, v)
          ()
        },
        (m1, m2) => {
          val it = m2.entrySet().iterator()
          while it.hasNext do
            val e = it.next()
            val v = e.getValue
            if v == null then throw new NullPointerException()
            val u = m1.get(e.getKey)
            if u != null then throw Collectors.duplicateKey(e.getKey, u, v)
            m1.put(e.getKey, v)
          m1
        },
        null, ID)
    def toMap[T, K, U](keyMapper: java.util.function.Function[? >: T, ? <: K], valueMapper: java.util.function.Function[? >: T, ? <: U], mergeFunction: java.util.function.BinaryOperator[U]): Collector[T, ?, java.util.Map[K, U]] =
      toMap[T, K, U, java.util.Map[K, U]](keyMapper, valueMapper, mergeFunction, () => new java.util.HashMap[K, U]())
    // A key mapped twice keeps what the merge function makes of the two values: a null result
    // removes it.
    def toMap[T, K, U, M <: java.util.Map[K, U]](keyMapper: java.util.function.Function[? >: T, ? <: K], valueMapper: java.util.function.Function[? >: T, ? <: U], mergeFunction: java.util.function.BinaryOperator[U], mapFactory: java.util.function.Supplier[M]): Collector[T, ?, M] =
      val keyOf = keyMapper.asInstanceOf[java.util.function.Function[T, K]]
      val valueOf = valueMapper.asInstanceOf[java.util.function.Function[T, U]]
      new CollectorImpl[T, M, M](
        mapFactory,
        (map, element) => { Collectors.merge(map, keyOf.apply(element), valueOf.apply(element), mergeFunction); () },
        (m1, m2) => {
          val it = m2.entrySet().iterator()
          while it.hasNext do
            val e = it.next()
            Collectors.merge(m1, e.getKey, e.getValue, mergeFunction)
          m1
        },
        null, ID)

    def groupingBy[T, K](classifier: java.util.function.Function[? >: T, ? <: K]): Collector[T, ?, java.util.Map[K, java.util.List[T]]] =
      groupingBy[T, K, Any, java.util.List[T]](classifier, toList[T]().asInstanceOf[Collector[T, Any, java.util.List[T]]])
    def groupingBy[T, K, A, D](classifier: java.util.function.Function[? >: T, ? <: K], downstream: Collector[? >: T, A, D]): Collector[T, ?, java.util.Map[K, D]] =
      groupingBy[T, K, D, A, java.util.Map[K, D]](classifier, () => new java.util.HashMap[K, D](), downstream)
    // The downstream collector's functions are asked for now; a null key is a
    // `NullPointerException`.
    def groupingBy[T, K, D, A, M <: java.util.Map[K, D]](classifier: java.util.function.Function[? >: T, ? <: K], mapFactory: java.util.function.Supplier[M], downstream: Collector[? >: T, A, D]): Collector[T, ?, M] =
      val down = downstream.asInstanceOf[Collector[T, A, D]]
      val downSupplier = down.supplier()
      val downAccumulator = down.accumulator()
      val downCombiner = down.combiner()
      val classify = classifier.asInstanceOf[java.util.function.Function[T, K]]
      val accumulate: java.util.function.BiConsumer[java.util.Map[K, A], T] = (m, t) =>
        val key = classify.apply(t)
        if key == null then throw new NullPointerException("element cannot be mapped to a null key")
        var container = m.get(key)
        if container == null then
          container = downSupplier.get()
          if container != null then m.put(key, container)
        downAccumulator.accept(container, t)
      val combine: java.util.function.BinaryOperator[java.util.Map[K, A]] = (m1, m2) =>
        val it = m2.entrySet().iterator()
        while it.hasNext do
          val e = it.next()
          merge(m1, e.getKey, e.getValue, downCombiner)
        m1
      val factory = mapFactory.asInstanceOf[java.util.function.Supplier[java.util.Map[K, A]]]
      if down.characteristics().contains(Collector.Characteristics.IDENTITY_FINISH) then
        new CollectorImpl[T, java.util.Map[K, A], M](factory, accumulate, combine, null, ID)
      else
        val downFinisher = down.finisher()
        val finish: java.util.function.Function[java.util.Map[K, A], M] = intermediate =>
          val it = intermediate.entrySet().iterator()
          val keys = new java.util.ArrayList[K]()
          while it.hasNext do keys.add(it.next().getKey)
          var i = 0
          while i < keys.size() do
            val k = keys.get(i)
            intermediate.put(k, downFinisher.apply(intermediate.get(k)).asInstanceOf[A])
            i += 1
          intermediate.asInstanceOf[M]
        new CollectorImpl[T, java.util.Map[K, A], M](factory, accumulate, combine, finish, NO_ID)

    private[stream] def duplicateKey(k: Any, u: Any, v: Any): IllegalStateException =
      new IllegalStateException("Duplicate key " + k + " (attempted merging values " + u + " and " + v + ")")
    // `Map.merge`: a null value or function refused; an absent key, or one mapped to null, takes
    // the value; a present one the function's result, removed where that is null.
    private[stream] def merge[K, V](map: java.util.Map[K, V], key: K, value: V, f: java.util.function.BiFunction[? >: V, ? >: V, ? <: V]): Unit =
      if value == null || f == null then throw new NullPointerException()
      val old = map.get(key)
      if old == null then map.put(key, value)
      else
        val v = f.asInstanceOf[java.util.function.BiFunction[V, V, V]].apply(old, value)
        if v == null then map.remove(key) else map.put(key, v)

  // ---- the pipeline ----

  // The JDK's `Sink`: a stage's consumer of elements, told before the first how many will come
  // (-1 when that is not known) and after the last that there are no more; a stage that can stop
  // the traversal early answers `cancellationRequested` before each element.
  private[java] abstract class Sink extends java.util.function.Consumer[Any]:
    def begin(size: Long): Unit = ()
    def end(): Unit = ()
    def cancellationRequested(): Boolean = false

  private[java] abstract class Chained(val downstream: Sink) extends Sink:
    override def begin(size: Long): Unit = downstream.begin(size)
    override def end(): Unit = downstream.end()
    override def cancellationRequested(): Boolean = downstream.cancellationRequested()

  // The operation a stage adds: its sink in front of the next stage's, given whether the pipeline
  // stops early (which `flatMap` asks) and the stage before it, whose flags `sorted` and
  // `distinct` read; and how it changes the pipeline's flags (`StreamOpFlag`): whether the
  // elements stop being as many as the source's (`unsized`), whether it stops the traversal
  // early, whether it orders the elements (1) or gives up their order (2), whether they are then
  // in their natural order (`sorting`) and each once (`distinctness`), 1 for yes, 2 for no
  // longer, 0 for as before. `size` is the number of elements out of `n` in, for a sized pipeline.
  private[java] abstract class Op(val unsized: Boolean, val shortCircuits: Boolean, val ordering: Int, val sorting: Int = 0, val distinctness: Int = 0):
    def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink
    def size(n: Long): Long = n

  // A stage (`AbstractPipeline`): the source stage holds the source until a use takes it, the
  // close action and the parallel flag; a later stage the operation it adds. Making a stage
  // takes the one it builds on.
  private[java] abstract class Pipe(val previous: Pipe, val op: Op, headFlags: Int):
    if previous != null then previous.link()
    val sourceStage: Pipe = if previous == null then this else previous.sourceStage
    private[java] var linked = false
    private[java] var source: java.util.Spliterator[Any] = null
    private[java] var supplier: java.util.function.Supplier[java.util.Spliterator[Any]] = null
    private[java] var closeAction: Runnable = null
    private[java] var parallelFlag = false
    val sized: Boolean = if previous == null then (headFlags & Pipes.SIZED) != 0 else previous.sized && !op.unsized
    val ordered: Boolean =
      if previous == null then (headFlags & Pipes.ORDERED) != 0
      else if op.ordering == 1 then true
      else if op.ordering == 2 then false
      else previous.ordered
    val shortCircuits: Boolean = previous != null && (previous.shortCircuits || op.shortCircuits)
    val sortedNatural: Boolean =
      if previous == null then (headFlags & java.util.Spliterator.SORTED) != 0
      else if op.sorting == 1 then true
      else if op.sorting == 2 then false
      else previous.sortedNatural
    val distinctElements: Boolean =
      if previous == null then (headFlags & java.util.Spliterator.DISTINCT) != 0
      else if op.distinctness == 1 then true
      else if op.distinctness == 2 then false
      else previous.distinctElements

    private[java] def link(): Unit =
      if linked then throw new IllegalStateException(Pipes.LINKED)
      linked = true

    // The source, for the use that takes it; none left once the source stage is closed.
    private[java] def sourceSpliterator(): java.util.Spliterator[Any] =
      val s = sourceStage
      if s.source != null then
        val out = s.source
        s.source = null
        out
      else if s.supplier != null then
        val out = s.supplier.get()
        s.supplier = null
        out
      else throw new IllegalStateException(Pipes.CONSUMED)

    // The number of elements, where the source knows its own and no stage changes it but by slicing.
    private[java] def exactOutputSize(sp: java.util.Spliterator[Any]): Long =
      if !sized then -1L else outSize(sp.getExactSizeIfKnown())
    private def outSize(n: Long): Long = if previous == null || n < 0 then n else op.size(previous.outSize(n))

    private[java] def wrapSink(sink: Sink, shorts: Boolean): Sink =
      var s = sink
      var p = this
      while p.previous != null do
        s = p.op.wrap(s, shorts, p.previous)
        p = p.previous
      s

    // The source's elements through a wrapped sink: all at once, or one at a time asking first
    // whether to go on where the pipeline stops early.
    private[java] def copyInto(wrapped: Sink, sp: java.util.Spliterator[Any], shorts: Boolean): Unit =
      wrapped.begin(sp.getExactSizeIfKnown())
      if shorts then
        while !wrapped.cancellationRequested() && sp.tryAdvance(wrapped) do ()
      else sp.forEachRemaining(wrapped)
      wrapped.end()

    // `opFlags`, where given, runs once the stage is taken and before the source is: what the
    // JDK asks a collector's characteristics for there.
    private[java] def evaluate[S <: Sink](terminal: S, terminalShorts: Boolean, opFlags: () => Any = null): S =
      link()
      if opFlags != null then opFlags()
      val sp = sourceSpliterator()
      val shorts = terminalShorts || shortCircuits
      copyInto(wrapSink(terminal, shorts), sp, shorts)
      terminal

    // `spliterator`: the source stage's own source, a later stage's elements on demand.
    private[java] def split(): java.util.Spliterator[Any] =
      link()
      if previous != null then new WrappingSpliterator(this)
      else
        val s = sourceStage
        if s.source != null then
          val out = s.source
          s.source = null
          out
        else if s.supplier != null then
          val supply = s.supplier
          s.supplier = null
          new LazySpliterator(supply)
        else throw new IllegalStateException(Pipes.CONSUMED)

    // `forEach`: the source stage hands its source the action, a later stage runs the pipeline.
    private[java] def forEachRaw(action: java.util.function.Consumer[Any]): Unit =
      if previous == null && !sourceStage.parallelFlag then
        link()
        sourceSpliterator().forEachRemaining(action)
      else
        if action == null then throw new NullPointerException()
        evaluate(new Sink { def accept(t: Any): Unit = action.accept(t) }, false)

    // `anyMatch` (0), `allMatch` (1), `noneMatch` (2).
    private[java] def matchRaw(kind: Int, test: Any => Boolean): Boolean = evaluate(new MatchSink(kind, test), true).value

    // `findFirst`: the first element, `found` false for none.
    private[java] def findRaw(): Found =
      val sink = new Found
      evaluate(sink, true)

    private[java] def foldRaw(seed: Any, f: (Any, Any) => Any): Any = evaluate(new FoldSink(() => seed, (s, t) => f(s, t)), false).state

    // A reduction without a seed: `found` false for no element.
    private[java] def reduceRaw(f: (Any, Any) => Any): Found = evaluate(new ReduceSink(f), false)

    private[java] def collectRaw(supply: () => Any, accumulate: (Any, Any) => Unit, opFlags: () => Any = null): Any =
      evaluate(new FoldSink(supply, (s, t) => { accumulate(s, t); s }), false, opFlags).state

    private[java] def countRaw(): Long =
      link()
      val sp = sourceSpliterator()
      val n = exactOutputSize(sp)
      if n != -1L then n
      else
        val sink = new CountSink
        copyInto(wrapSink(sink, shortCircuits), sp, shortCircuits)
        sink.count

    // `toArray`: a sized pipeline's elements go into the array made for its size before the
    // traversal; another's are gathered, then copied into the array made for their number.
    private[java] def toArrayRaw(make: Int => Array[Any]): Array[Any] =
      link()
      val sp = sourceSpliterator()
      val n = exactOutputSize(sp)
      if n >= 0 && n < Pipes.MAX_ARRAY_SIZE then
        val array = make(n.toInt)
        val sink = new Sink:
          var cur = 0
          override def begin(size: Long): Unit =
            if size != array.length then
              throw new IllegalStateException("Begin size " + size + " is not equal to fixed size " + array.length)
            cur = 0
          def accept(t: Any): Unit =
            if cur < array.length then
              array(cur) = t
              cur += 1
            else throw new IllegalStateException("Accept exceeded fixed size of " + array.length)
          override def end(): Unit =
            if cur < array.length then
              throw new IllegalStateException("End size " + cur + " is less than fixed size " + array.length)
        copyInto(wrapSink(sink, shortCircuits), sp, shortCircuits)
        array
      else
        val items = new java.util.ArrayList[Any]()
        val sink = new Sink:
          override def begin(size: Long): Unit = items.clear()
          def accept(t: Any): Unit = items.add(t)
        copyInto(wrapSink(sink, shortCircuits), sp, shortCircuits)
        val count = items.size()
        val array = make(count)
        if count > array.length then throw new IndexOutOfBoundsException("does not fit")
        var i = 0
        while i < count do
          array(i) = items.get(i)
          i += 1
        array

    private[java] def addClose(closeHandler: Runnable): Unit =
      if linked then throw new IllegalStateException(Pipes.LINKED)
      if closeHandler == null then throw new NullPointerException()
      val s = sourceStage
      s.closeAction = if s.closeAction == null then closeHandler else Pipes.both(s.closeAction, closeHandler)

    private[java] def closeRaw(): Unit =
      linked = true
      if previous == null then
        source = null
        supplier = null
      val s = sourceStage
      val action = s.closeAction
      if action != null then
        s.closeAction = null
        action.run()

    // `unordered`: a stage giving up the order, or this one where there is none.
    private[java] def unorderedOp: Op = new Op(false, false, 2):
      def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink = sink

  // The terminal operations' sinks. `findFirst`'s: the first element, `found` false for none.
  private[java] class Found extends Sink:
    var found = false
    var value: Any = null
    def accept(t: Any): Unit =
      if !found then
        found = true
        value = t
    override def cancellationRequested(): Boolean = found

  private[java] final class ReduceSink(f: (Any, Any) => Any) extends Found:
    override def begin(size: Long): Unit =
      found = false
      value = null
    override def accept(t: Any): Unit =
      if !found then
        found = true
        value = t
      else value = f(value, t)
    override def cancellationRequested(): Boolean = false

  // A reduction from a seed, and `collect`'s: the state made at the start, each element folded in.
  private[java] final class FoldSink(seed: () => Any, f: (Any, Any) => Any) extends Sink:
    var state: Any = null
    override def begin(size: Long): Unit = state = seed()
    def accept(t: Any): Unit = state = f(state, t)

  private[java] final class CountSink extends Sink:
    var count = 0L
    override def begin(size: Long): Unit = count = 0L
    def accept(t: Any): Unit = count += 1L

  // `anyMatch` (0), `allMatch` (1), `noneMatch` (2): stops at the first element deciding it.
  private[java] final class MatchSink(kind: Int, test: Any => Boolean) extends Sink:
    private val stopOn = kind != 1
    private val stopResult = kind == 0
    private var stop = false
    var value: Boolean = !stopResult
    def accept(t: Any): Unit =
      if !stop && test(t) == stopOn then
        stop = true
        value = stopResult
    override def cancellationRequested(): Boolean = stop

  // A stage of objects.
  private[java] final class RefPipe[T](previous: Pipe, op: Op, headFlags: Int) extends Pipe(previous, op, headFlags), Stream[T]:
    def iterator(): java.util.Iterator[T] = java.util.Spliterators.iterator(spliterator())
    def spliterator(): java.util.Spliterator[T] = split().asInstanceOf[java.util.Spliterator[T]]
    def isParallel(): Boolean = sourceStage.parallelFlag
    def sequential(): Stream[T] =
      sourceStage.parallelFlag = false
      this
    def parallel(): Stream[T] =
      sourceStage.parallelFlag = true
      this
    def unordered(): Stream[T] = if !ordered then this else new RefPipe[T](this, unorderedOp, 0)
    def onClose(closeHandler: Runnable): Stream[T] =
      addClose(closeHandler)
      this
    def close(): Unit = closeRaw()

    def filter(predicate: java.util.function.Predicate[? >: T]): Stream[T] =
      if predicate == null then throw new NullPointerException()
      val p = predicate.asInstanceOf[java.util.function.Predicate[Any]]
      new RefPipe[T](this, Ops.filter(t => p.test(t)), 0)
    def map[R](mapper: java.util.function.Function[? >: T, ? <: R]): Stream[R] =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.Function[Any, Any]]
      new RefPipe[R](this, Ops.map(t => f.apply(t)), 0)
    def mapToInt(mapper: java.util.function.ToIntFunction[? >: T]): IntStream =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.ToIntFunction[Any]]
      new IntPipe(this, Ops.map(t => f.applyAsInt(t)), 0)
    def mapToLong(mapper: java.util.function.ToLongFunction[? >: T]): LongStream =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.ToLongFunction[Any]]
      new LongPipe(this, Ops.map(t => f.applyAsLong(t)), 0)
    def mapToDouble(mapper: java.util.function.ToDoubleFunction[? >: T]): DoubleStream =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.ToDoubleFunction[Any]]
      new DoublePipe(this, Ops.map(t => f.applyAsDouble(t)), 0)
    def flatMap[R](mapper: java.util.function.Function[? >: T, ? <: Stream[? <: R]]): Stream[R] =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.Function[Any, Any]]
      new RefPipe[R](this, Ops.flatMap(t => f.apply(t)), 0)
    def flatMapToInt(mapper: java.util.function.Function[? >: T, ? <: IntStream]): IntStream =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.Function[Any, Any]]
      new IntPipe(this, Ops.flatMap(t => f.apply(t)), 0)
    def flatMapToLong(mapper: java.util.function.Function[? >: T, ? <: LongStream]): LongStream =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.Function[Any, Any]]
      new LongPipe(this, Ops.flatMap(t => f.apply(t)), 0)
    def flatMapToDouble(mapper: java.util.function.Function[? >: T, ? <: DoubleStream]): DoubleStream =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.Function[Any, Any]]
      new DoublePipe(this, Ops.flatMap(t => f.apply(t)), 0)
    def mapMulti[R](mapper: java.util.function.BiConsumer[? >: T, ? >: java.util.function.Consumer[R]]): Stream[R] =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.BiConsumer[Any, Any]]
      new RefPipe[R](this, Ops.mapMulti((t, sink) => f.accept(t, sink)), 0)
    def mapMultiToInt(mapper: java.util.function.BiConsumer[? >: T, ? >: java.util.function.IntConsumer]): IntStream =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.BiConsumer[Any, Any]]
      new IntPipe(this, Ops.mapMulti((t, sink) => f.accept(t, Pipes.intConsumer(sink))), 0)
    def mapMultiToLong(mapper: java.util.function.BiConsumer[? >: T, ? >: java.util.function.LongConsumer]): LongStream =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.BiConsumer[Any, Any]]
      new LongPipe(this, Ops.mapMulti((t, sink) => f.accept(t, Pipes.longConsumer(sink))), 0)
    def mapMultiToDouble(mapper: java.util.function.BiConsumer[? >: T, ? >: java.util.function.DoubleConsumer]): DoubleStream =
      if mapper == null then throw new NullPointerException()
      val f = mapper.asInstanceOf[java.util.function.BiConsumer[Any, Any]]
      new DoublePipe(this, Ops.mapMulti((t, sink) => f.accept(t, Pipes.doubleConsumer(sink))), 0)
    def distinct(): Stream[T] = new RefPipe[T](this, Ops.distinct, 0)
    def sorted(): Stream[T] = new RefPipe[T](this, Ops.sorted((a, b) => java.util.SortedOps.compareNatural(a, b), true), 0)
    // As the JDK's, the comparator is checked once the stage is made.
    def sorted(comparator: java.util.Comparator[? >: T]): Stream[T] =
      val c = comparator.asInstanceOf[java.util.Comparator[Any]]
      val out = new RefPipe[T](this, Ops.sorted((a, b) => c.compare(a, b), false), 0)
      if comparator == null then throw new NullPointerException()
      out
    def peek(action: java.util.function.Consumer[? >: T]): Stream[T] =
      if action == null then throw new NullPointerException()
      val f = action.asInstanceOf[java.util.function.Consumer[Any]]
      new RefPipe[T](this, Ops.peek(t => f.accept(t)), 0)
    def limit(maxSize: Long): Stream[T] =
      if maxSize < 0 then throw new IllegalArgumentException(java.lang.Long.toString(maxSize))
      new RefPipe[T](this, Ops.slice(0, maxSize), 0)
    def skip(n: Long): Stream[T] =
      if n < 0 then throw new IllegalArgumentException(java.lang.Long.toString(n))
      if n == 0 then this else new RefPipe[T](this, Ops.slice(n, -1), 0)
    def takeWhile(predicate: java.util.function.Predicate[? >: T]): Stream[T] =
      if predicate == null then throw new NullPointerException()
      val p = predicate.asInstanceOf[java.util.function.Predicate[Any]]
      new RefPipe[T](this, Ops.takeWhile(t => p.test(t)), 0)
    def dropWhile(predicate: java.util.function.Predicate[? >: T]): Stream[T] =
      if predicate == null then throw new NullPointerException()
      val p = predicate.asInstanceOf[java.util.function.Predicate[Any]]
      new RefPipe[T](this, Ops.dropWhile(t => p.test(t)), 0)

    def forEach(action: java.util.function.Consumer[? >: T]): Unit = forEachRaw(action.asInstanceOf[java.util.function.Consumer[Any]])
    def forEachOrdered(action: java.util.function.Consumer[? >: T]): Unit = forEachRaw(action.asInstanceOf[java.util.function.Consumer[Any]])
    def toArray(): Array[AnyRef] = toArrayRaw(n => new Array[AnyRef](n).asInstanceOf[Array[Any]]).asInstanceOf[Array[AnyRef]]
    def toArray[A](generator: java.util.function.IntFunction[Array[A & AnyRef]]): Array[A & AnyRef] =
      toArrayRaw(n => generator.apply(n).asInstanceOf[Array[Any]]).asInstanceOf[Array[A & AnyRef]]
    def reduce(identity: T, accumulator: java.util.function.BinaryOperator[T]): T =
      if accumulator == null then throw new NullPointerException()
      val f = accumulator.asInstanceOf[java.util.function.BinaryOperator[Any]]
      foldRaw(identity, (a, b) => f.apply(a, b)).asInstanceOf[T]
    def reduce(accumulator: java.util.function.BinaryOperator[T]): java.util.Optional[T] =
      if accumulator == null then throw new NullPointerException()
      val f = accumulator.asInstanceOf[java.util.function.BinaryOperator[Any]]
      val r = reduceRaw((a, b) => f.apply(a, b))
      if r.found then java.util.Optional.of[T](r.value.asInstanceOf[T]) else java.util.Optional.empty[T]()
    def reduce[U](identity: U, accumulator: java.util.function.BiFunction[U, ? >: T, U], combiner: java.util.function.BinaryOperator[U]): U =
      if accumulator == null then throw new NullPointerException()
      if combiner == null then throw new NullPointerException()
      val f = accumulator.asInstanceOf[java.util.function.BiFunction[Any, Any, Any]]
      foldRaw(identity, (a, b) => f.apply(a, b)).asInstanceOf[U]
    def collect[R](supplier: java.util.function.Supplier[R], accumulator: java.util.function.BiConsumer[R, ? >: T], combiner: java.util.function.BiConsumer[R, R]): R =
      if supplier == null || accumulator == null || combiner == null then throw new NullPointerException()
      val f = accumulator.asInstanceOf[java.util.function.BiConsumer[Any, Any]]
      collectRaw(() => supplier.get(), (r, t) => f.accept(r, t)).asInstanceOf[R]
    // The collector's functions are asked for before the stream is used; whether it is
    // `UNORDERED` once the stream is taken and before its source is (`ReduceOps.makeRef`'s
    // `getOpFlags`), the answer of no use to a sequential stream; and whether it finishes by
    // identity after the elements.
    def collect[R, A](collector: Collector[? >: T, A, R]): R =
      if collector == null then throw new NullPointerException()
      val c = collector.asInstanceOf[Collector[Any, Any, Any]]
      val supply = c.supplier()
      val accumulate = c.accumulator()
      c.combiner()
      val container = collectRaw(() => supply.get(), (r, t) => accumulate.accept(r, t), () => Pipes.has(c, Collector.Characteristics.UNORDERED))
      if Pipes.has(c, Collector.Characteristics.IDENTITY_FINISH) then container.asInstanceOf[R]
      else c.finisher().apply(container).asInstanceOf[R]
    // An unmodifiable list, as `Stream.toList`'s.
    def toList(): java.util.List[T] = new FixedList[T](toArrayRaw(n => new Array[Any](n)))
    def min(comparator: java.util.Comparator[? >: T]): java.util.Optional[T] = reduce(java.util.function.BinaryOperator.minBy[T](comparator))
    def max(comparator: java.util.Comparator[? >: T]): java.util.Optional[T] = reduce(java.util.function.BinaryOperator.maxBy[T](comparator))
    def count(): Long = countRaw()
    def anyMatch(predicate: java.util.function.Predicate[? >: T]): Boolean =
      if predicate == null then throw new NullPointerException()
      val p = predicate.asInstanceOf[java.util.function.Predicate[Any]]
      matchRaw(0, t => p.test(t))
    def allMatch(predicate: java.util.function.Predicate[? >: T]): Boolean =
      if predicate == null then throw new NullPointerException()
      val p = predicate.asInstanceOf[java.util.function.Predicate[Any]]
      matchRaw(1, t => p.test(t))
    def noneMatch(predicate: java.util.function.Predicate[? >: T]): Boolean =
      if predicate == null then throw new NullPointerException()
      val p = predicate.asInstanceOf[java.util.function.Predicate[Any]]
      matchRaw(2, t => p.test(t))
    // A null element found is a `NullPointerException`, as `Optional.of` makes it.
    def findFirst(): java.util.Optional[T] =
      val r = findRaw()
      if r.found then java.util.Optional.of[T](r.value.asInstanceOf[T]) else java.util.Optional.empty[T]()
    def findAny(): java.util.Optional[T] = findFirst()

  // A stage of `int`s, `long`s or `double`s: the elements boxed between the stages, the results
  // the primitive ones.
  private[java] final class IntPipe(previous: Pipe, op: Op, headFlags: Int) extends Pipe(previous, op, headFlags), IntStream:
    def iterator(): java.util.PrimitiveIterator.OfInt =
      val it = java.util.Spliterators.iterator(split())
      new java.util.PrimitiveIterator.OfInt:
        def hasNext: Boolean = it.hasNext
        def nextInt(): Int = it.next().asInstanceOf[Int]
    def spliterator(): java.util.Spliterator.OfInt = new IntSpliterator(split())
    def isParallel(): Boolean = sourceStage.parallelFlag
    def sequential(): IntStream =
      sourceStage.parallelFlag = false
      this
    def parallel(): IntStream =
      sourceStage.parallelFlag = true
      this
    def unordered(): IntStream = if !ordered then this else new IntPipe(this, unorderedOp, 0)
    def onClose(closeHandler: Runnable): IntStream =
      addClose(closeHandler)
      this
    def close(): Unit = closeRaw()
    def forEach(action: java.util.function.IntConsumer): Unit =
      forEachRaw(if action == null then null else (t: Any) => action.accept(t.asInstanceOf[Int]))
    def toArray(): Array[Int] =
      val items = toArrayRaw(n => new Array[Any](n))
      val out = new Array[Int](items.length)
      var i = 0
      while i < items.length do
        out(i) = items(i).asInstanceOf[Int]
        i += 1
      out
    def sum(): Int = foldRaw(0, (a, b) => a.asInstanceOf[Int] + b.asInstanceOf[Int]).asInstanceOf[Int]
    def min(): java.util.OptionalInt =
      val r = reduceRaw((a, b) => Math.min(a.asInstanceOf[Int], b.asInstanceOf[Int]))
      if r.found then java.util.OptionalInt.of(r.value.asInstanceOf[Int]) else java.util.OptionalInt.empty()
    def max(): java.util.OptionalInt =
      val r = reduceRaw((a, b) => Math.max(a.asInstanceOf[Int], b.asInstanceOf[Int]))
      if r.found then java.util.OptionalInt.of(r.value.asInstanceOf[Int]) else java.util.OptionalInt.empty()
    def count(): Long = countRaw()
    // The sum in a `long`, so that it does not overflow.
    def average(): java.util.OptionalDouble =
      val avg = collectRaw(() => new Array[Long](2), (a, t) => {
        val ll = a.asInstanceOf[Array[Long]]
        ll(0) = ll(0) + 1L
        ll(1) = ll(1) + t.asInstanceOf[Int].toLong
      }).asInstanceOf[Array[Long]]
      if avg(0) > 0 then java.util.OptionalDouble.of(avg(1).toDouble / avg(0)) else java.util.OptionalDouble.empty()
    def boxed(): Stream[java.lang.Integer] = new RefPipe[java.lang.Integer](this, Ops.map(t => t), 0)

  private[java] final class LongPipe(previous: Pipe, op: Op, headFlags: Int) extends Pipe(previous, op, headFlags), LongStream:
    def iterator(): java.util.PrimitiveIterator.OfLong =
      val it = java.util.Spliterators.iterator(split())
      new java.util.PrimitiveIterator.OfLong:
        def hasNext: Boolean = it.hasNext
        def nextLong(): Long = it.next().asInstanceOf[Long]
    def spliterator(): java.util.Spliterator.OfLong = new LongSpliterator(split())
    def isParallel(): Boolean = sourceStage.parallelFlag
    def sequential(): LongStream =
      sourceStage.parallelFlag = false
      this
    def parallel(): LongStream =
      sourceStage.parallelFlag = true
      this
    def unordered(): LongStream = if !ordered then this else new LongPipe(this, unorderedOp, 0)
    def onClose(closeHandler: Runnable): LongStream =
      addClose(closeHandler)
      this
    def close(): Unit = closeRaw()
    def forEach(action: java.util.function.LongConsumer): Unit =
      forEachRaw(if action == null then null else (t: Any) => action.accept(t.asInstanceOf[Long]))
    def toArray(): Array[Long] =
      val items = toArrayRaw(n => new Array[Any](n))
      val out = new Array[Long](items.length)
      var i = 0
      while i < items.length do
        out(i) = items(i).asInstanceOf[Long]
        i += 1
      out
    def sum(): Long = foldRaw(0L, (a, b) => a.asInstanceOf[Long] + b.asInstanceOf[Long]).asInstanceOf[Long]
    def min(): java.util.OptionalLong =
      val r = reduceRaw((a, b) => Math.min(a.asInstanceOf[Long], b.asInstanceOf[Long]))
      if r.found then java.util.OptionalLong.of(r.value.asInstanceOf[Long]) else java.util.OptionalLong.empty()
    def max(): java.util.OptionalLong =
      val r = reduceRaw((a, b) => Math.max(a.asInstanceOf[Long], b.asInstanceOf[Long]))
      if r.found then java.util.OptionalLong.of(r.value.asInstanceOf[Long]) else java.util.OptionalLong.empty()
    def count(): Long = countRaw()
    def average(): java.util.OptionalDouble =
      val avg = collectRaw(() => new Array[Long](2), (a, t) => {
        val ll = a.asInstanceOf[Array[Long]]
        ll(0) = ll(0) + 1L
        ll(1) = ll(1) + t.asInstanceOf[Long]
      }).asInstanceOf[Array[Long]]
      if avg(0) > 0 then java.util.OptionalDouble.of(avg(1).toDouble / avg(0)) else java.util.OptionalDouble.empty()
    def boxed(): Stream[java.lang.Long] = new RefPipe[java.lang.Long](this, Ops.map(t => t), 0)

  private[java] final class DoublePipe(previous: Pipe, op: Op, headFlags: Int) extends Pipe(previous, op, headFlags), DoubleStream:
    def iterator(): java.util.PrimitiveIterator.OfDouble =
      val it = java.util.Spliterators.iterator(split())
      new java.util.PrimitiveIterator.OfDouble:
        def hasNext: Boolean = it.hasNext
        def nextDouble(): Double = it.next().asInstanceOf[Double]
    def spliterator(): java.util.Spliterator.OfDouble = new DoubleSpliterator(split())
    def isParallel(): Boolean = sourceStage.parallelFlag
    def sequential(): DoubleStream =
      sourceStage.parallelFlag = false
      this
    def parallel(): DoubleStream =
      sourceStage.parallelFlag = true
      this
    def unordered(): DoubleStream = if !ordered then this else new DoublePipe(this, unorderedOp, 0)
    def onClose(closeHandler: Runnable): DoubleStream =
      addClose(closeHandler)
      this
    def close(): Unit = closeRaw()
    def forEach(action: java.util.function.DoubleConsumer): Unit =
      forEachRaw(if action == null then null else (t: Any) => action.accept(t.asInstanceOf[Double]))
    def toArray(): Array[Double] =
      val items = toArrayRaw(n => new Array[Any](n))
      val out = new Array[Double](items.length)
      var i = 0
      while i < items.length do
        out(i) = items(i).asInstanceOf[Double]
        i += 1
      out
    // Kahan's compensated sum, as the JDK's (`Collectors.sumWithCompensation`), with the simple
    // sum kept for infinities of one sign.
    def sum(): Double =
      val s = collectRaw(() => new Array[Double](3), (a, t) => {
        val ll = a.asInstanceOf[Array[Double]]
        val d = t.asInstanceOf[Double]
        Pipes.sumWithCompensation(ll, d)
        ll(2) = ll(2) + d
      }).asInstanceOf[Array[Double]]
      Pipes.computeFinalSum(s)
    def min(): java.util.OptionalDouble =
      val r = reduceRaw((a, b) => java.lang.Math.min(a.asInstanceOf[Double], b.asInstanceOf[Double]))
      if r.found then java.util.OptionalDouble.of(r.value.asInstanceOf[Double]) else java.util.OptionalDouble.empty()
    def max(): java.util.OptionalDouble =
      val r = reduceRaw((a, b) => java.lang.Math.max(a.asInstanceOf[Double], b.asInstanceOf[Double]))
      if r.found then java.util.OptionalDouble.of(r.value.asInstanceOf[Double]) else java.util.OptionalDouble.empty()
    def count(): Long = countRaw()
    def average(): java.util.OptionalDouble =
      val avg = collectRaw(() => new Array[Double](4), (a, t) => {
        val ll = a.asInstanceOf[Array[Double]]
        val d = t.asInstanceOf[Double]
        ll(2) = ll(2) + 1.0
        Pipes.sumWithCompensation(ll, d)
        ll(3) = ll(3) + d
      }).asInstanceOf[Array[Double]]
      if avg(2) > 0 then java.util.OptionalDouble.of(Pipes.computeFinalSum(avg) / avg(2)) else java.util.OptionalDouble.empty()
    def boxed(): Stream[java.lang.Double] = new RefPipe[java.lang.Double](this, Ops.map(t => t), 0)

  // The operations' sinks, as the JDK's stages make them.
  private[java] object Ops:
    def map(f: Any => Any): Op = new Op(false, false, 0, 2, 2):
      def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink = new Chained(sink):
        def accept(t: Any): Unit = downstream.accept(f(t))
    def filter(p: Any => Boolean): Op = new Op(true, false, 0):
      def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink = new Chained(sink):
        override def begin(size: Long): Unit = downstream.begin(-1)
        def accept(t: Any): Unit = if p(t) then downstream.accept(t)
    def peek(f: Any => Unit): Op = new Op(false, false, 0):
      def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink = new Chained(sink):
        def accept(t: Any): Unit =
          f(t)
          downstream.accept(t)
    // Each element's stream pushed downstream and closed after; a null one is empty. Where the
    // pipeline stops early, the inner stream stops too once the downstream asks.
    def flatMap(f: Any => Any): Op = new Op(true, false, 0, 2, 2):
      def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink = new Sink:
        var cancel = false
        override def begin(size: Long): Unit = sink.begin(-1)
        override def end(): Unit = sink.end()
        def accept(t: Any): Unit =
          val result = f(t)
          Pipes.closing(result.asInstanceOf[AutoCloseable]) {
            if result != null then
              val inner = result.asInstanceOf[Pipe]
              inner.sourceStage.parallelFlag = false
              if shorts then
                inner.matchRaw(1, x =>
                  if !cancel then
                    sink.accept(x)
                    cancel = cancel || sink.cancellationRequested()
                    !cancel
                  else false)
              else inner.forEachRaw((x: Any) => sink.accept(x))
          }
        override def cancellationRequested(): Boolean =
          cancel = cancel || sink.cancellationRequested()
          cancel
    def mapMulti(f: (Any, Sink) => Unit): Op = new Op(true, false, 0, 2, 2):
      def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink = new Chained(sink):
        override def begin(size: Long): Unit = downstream.begin(-1)
        def accept(t: Any): Unit = f(t, downstream)
    // Nothing to do after a stage giving each element once; after one in natural order, an
    // element unlike the one before; otherwise one not seen yet.
    def distinct: Op = new Op(true, false, 0, 0, 1):
      def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink =
        if upstream.distinctElements then sink
        else if upstream.sortedNatural then
          new Chained(sink):
            var seenNull = false
            var lastSeen: Any = null
            override def begin(size: Long): Unit =
              seenNull = false
              lastSeen = null
              downstream.begin(-1)
            override def end(): Unit =
              seenNull = false
              lastSeen = null
              downstream.end()
            def accept(t: Any): Unit =
              if t == null then
                if !seenNull then
                  seenNull = true
                  lastSeen = null
                  downstream.accept(null)
              else if lastSeen == null || !t.equals(lastSeen) then
                lastSeen = t
                downstream.accept(t)
        else
          new Chained(sink):
            var seen: java.util.HashSet[Any] = null
            override def begin(size: Long): Unit =
              seen = new java.util.HashSet[Any]()
              downstream.begin(-1)
            override def end(): Unit =
              seen = null
              downstream.end()
            def accept(t: Any): Unit = if seen.add(t) then downstream.accept(t)
    // All the elements, then sorted (stably), then pushed on; asking first whether to go on
    // where a later stage asked during the gathering. In natural order (`natural`), nothing to
    // do after a stage already in it.
    def sorted(compare: (Any, Any) => Int, natural: Boolean): Op = new Op(false, false, 1, if natural then 1 else 2, 0):
      def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink =
        if natural && upstream.sortedNatural then sink else sortingSink(sink, compare)
    private def sortingSink(sink: Sink, compare: (Any, Any) => Int): Sink = new Chained(sink):
        var items: java.util.ArrayList[Any] = null
        var asked = false
        override def begin(size: Long): Unit =
          if size >= Pipes.MAX_ARRAY_SIZE then throw new IllegalArgumentException("Stream size exceeds max array size")
          items = new java.util.ArrayList[Any]()
        def accept(t: Any): Unit = items.add(t)
        override def cancellationRequested(): Boolean =
          asked = true
          false
        override def end(): Unit =
          val array = new Array[Any](items.size())
          var i = 0
          while i < array.length do
            array(i) = items.get(i)
            i += 1
          items = null
          java.util.Arrays.sort(array, (a, b) => compare(a, b))
          downstream.begin(array.length)
          i = 0
          while i < array.length && !(asked && downstream.cancellationRequested()) do
            downstream.accept(array(i))
            i += 1
          downstream.end()
    // `skip` elements dropped, then at most `limit` (-1 for no limit) passed on.
    def slice(skip: Long, limit: Long): Op = new Op(false, limit != -1, 0):
      private val normalized = if limit >= 0 then limit else Long.MaxValue
      override def size(n: Long): Long = Pipes.calcSize(n, skip, normalized)
      def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink = new Chained(sink):
        var n = skip
        var m = normalized
        override def begin(size: Long): Unit = downstream.begin(Pipes.calcSize(size, skip, m))
        def accept(t: Any): Unit =
          if n == 0 then
            if m > 0 then
              m -= 1
              downstream.accept(t)
          else n -= 1
        override def cancellationRequested(): Boolean = m == 0 || downstream.cancellationRequested()
    def takeWhile(p: Any => Boolean): Op = new Op(true, true, 0):
      def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink = new Chained(sink):
        var take = true
        override def begin(size: Long): Unit = downstream.begin(-1)
        def accept(t: Any): Unit =
          if take then
            take = p(t)
            if take then downstream.accept(t)
        override def cancellationRequested(): Boolean = !take || downstream.cancellationRequested()
    def dropWhile(p: Any => Boolean): Op = new Op(true, false, 0):
      def wrap(sink: Sink, shorts: Boolean, upstream: Pipe): Sink = new Chained(sink):
        var take = false
        def accept(t: Any): Unit =
          if !take then take = !p(t)
          if take then downstream.accept(t)

  private[java] object Pipes:
    final val LINKED = "stream has already been operated upon or closed"
    final val CONSUMED = "source already consumed or closed"
    final val MAX_ARRAY_SIZE = Int.MaxValue - 8
    final val ORDERED = java.util.Spliterator.ORDERED
    final val SIZED = java.util.Spliterator.SIZED
    // An array's, and an empty stream's, which has no order.
    final val ARRAY = java.util.Spliterator.ORDERED | java.util.Spliterator.IMMUTABLE | java.util.Spliterator.SIZED | java.util.Spliterator.SUBSIZED
    final val EMPTY = java.util.Spliterator.SIZED | java.util.Spliterator.SUBSIZED

    // A source's flags (`StreamOpFlag.fromCharacteristics`): sorted only in its natural order.
    def flagsOf(sp: java.util.Spliterator[?]): Int =
      val c = sp.characteristics()
      if (c & java.util.Spliterator.SORTED) != 0 && sp.getComparator() != null then c & ~java.util.Spliterator.SORTED else c
    // The elements of a varargs call: a spliced array itself, which the stream reads when it
    // runs, as the JDK's `Arrays.stream` of a Java varargs array; otherwise a copy.
    def arrayOf(values: Seq[?]): Array[Any] = values match
      case a: scala.collection.immutable.ArraySeq[?] => a.unsafeArray.asInstanceOf[Array[Any]]
      case _ => untaggedArray(iterableToArray(values)).asInstanceOf[Array[Any]]
    def refs[T](items: Array[Any]): Stream[T] = StreamSupport.stream(new java.util.ArraySpliterator[T](items, ARRAY), false)
    def ints(items: Array[Any], flags: Int = ARRAY): IntStream =
      val head = new IntPipe(null, null, flags)
      head.source = new java.util.ArraySpliterator[Any](items, flags)
      head
    def longs(items: Array[Any], flags: Int = ARRAY): LongStream =
      val head = new LongPipe(null, null, flags)
      head.source = new java.util.ArraySpliterator[Any](items, flags)
      head
    def doubles(items: Array[Any], flags: Int = ARRAY): DoubleStream =
      val head = new DoublePipe(null, null, flags)
      head.source = new java.util.ArraySpliterator[Any](items, flags)
      head
    // A stream over an iterator, of no known size: `Files.list`'s and `Files.walk`'s.
    def iterated[T](it: java.util.Iterator[T]): Stream[T] =
      StreamSupport.stream(java.util.Spliterators.spliteratorUnknownSize(it, java.util.Spliterator.DISTINCT), false)

    // Whether a collector's characteristics hold `trait`, a null set refused.
    def has(c: Collector[?, ?, ?], `trait`: Collector.Characteristics): Boolean =
      val cs = c.characteristics()
      if cs == null then throw new NullPointerException()
      cs.contains(`trait`)

    // The downstream sink as the consumer `mapMultiToInt` and the others hand their mapper.
    def intConsumer(sink: Sink): java.util.function.IntConsumer = v => sink.accept(v)
    def longConsumer(sink: Sink): java.util.function.LongConsumer = v => sink.accept(v)
    def doubleConsumer(sink: Sink): java.util.function.DoubleConsumer = v => sink.accept(v)

    def calcSize(size: Long, skip: Long, limit: Long): Long =
      if size >= 0 then Math.max(0L, Math.min(size - skip, limit)) else -1L

    // `Streams.composeWithExceptions`: the second runs whether the first throws or not, its
    // exception suppressed by the first's.
    def both(a: Runnable, b: Runnable): Runnable = () => {
      try a.run()
      catch
        case e1: Throwable =>
          try b.run()
          catch case e2: Throwable => if e2 ne e1 then e1.addSuppressed(e2)
          throw e1
      b.run()
    }
    // `Streams.composedClose`: both streams closed, the second's exception suppressed by the first's.
    def closeBoth(a: BaseStream[?, ?], b: BaseStream[?, ?]): Runnable = () => {
      try a.close()
      catch
        case e1: Throwable =>
          try b.close()
          catch case e2: Throwable => if e2 ne e1 then e1.addSuppressed(e2)
          throw e1
      b.close()
    }
    // `try (resource) { body }`: the resource, where there is one, closed after the body, an
    // exception of the close's suppressed by the body's.
    def closing(resource: AutoCloseable)(body: => Unit): Unit =
      var primary: Throwable = null
      try body
      catch
        case t: Throwable =>
          primary = t
          throw t
      finally
        if resource != null then
          if primary != null then
            try resource.close()
            catch case t2: Throwable => if t2 ne primary then primary.addSuppressed(t2)
          else resource.close()

    def sumWithCompensation(intermediateSum: Array[Double], value: Double): Unit =
      val tmp = value - intermediateSum(1)
      val sum = intermediateSum(0)
      val velvel = sum + tmp
      intermediateSum(1) = (velvel - sum) - tmp
      intermediateSum(0) = velvel
    def computeFinalSum(summands: Array[Double]): Double =
      val tmp = summands(0) - summands(1)
      val simpleSum = summands(summands.length - 1)
      if java.lang.Double.isNaN(tmp) && java.lang.Double.isInfinite(simpleSum) then simpleSum else tmp

  // ---- sources ----

  // A later stage's elements one at a time (`StreamSpliterators.WrappingSpliterator`): the source
  // taken at the first use, its elements pushed through the stages until one comes out (several
  // where a stage gives several, all of a sorted stage's at the end), the rest kept for the next.
  private[java] final class WrappingSpliterator(stage: Pipe) extends java.util.Spliterator[Any]:
    private var sp: java.util.Spliterator[Any] = null
    private var buffer: java.util.ArrayList[Any] = null
    private var bufferSink: Sink = null
    private var next = 0
    private var finished = false
    private def init(): Unit = if sp == null then sp = stage.sourceSpliterator()
    private def fill(): Boolean =
      var out = true
      while out && buffer.size() == 0 do
        if bufferSink.cancellationRequested() || !sp.tryAdvance(bufferSink) then
          if finished then out = false
          else
            bufferSink.end()
            finished = true
      out
    private def doAdvance(): Boolean =
      if buffer == null then
        if finished then false
        else
          init()
          val b = new java.util.ArrayList[Any]()
          buffer = b
          bufferSink = stage.wrapSink(new Sink { def accept(t: Any): Unit = b.add(t) }, stage.shortCircuits)
          next = 0
          bufferSink.begin(sp.getExactSizeIfKnown())
          fill()
      else
        next += 1
        if next < buffer.size() then true
        else
          next = 0
          buffer.clear()
          fill()
    def tryAdvance(action: java.util.function.Consumer[? >: Any]): Boolean =
      if action == null then throw new NullPointerException()
      val has = doAdvance()
      if has then action.asInstanceOf[java.util.function.Consumer[Any]].accept(buffer.get(next))
      has
    override def forEachRemaining(action: java.util.function.Consumer[? >: Any]): Unit =
      if buffer == null && !finished then
        if action == null then throw new NullPointerException()
        init()
        val f = action.asInstanceOf[java.util.function.Consumer[Any]]
        stage.copyInto(stage.wrapSink(new Sink { def accept(t: Any): Unit = f.accept(t) }, stage.shortCircuits), sp, stage.shortCircuits)
        finished = true
      else
        while tryAdvance(action) do ()
    def trySplit(): java.util.Spliterator[Any] = null
    def estimateSize(): Long =
      val n = getExactSizeIfKnown()
      if n == -1L then sp.estimateSize() else n
    override def getExactSizeIfKnown(): Long =
      init()
      stage.exactOutputSize(sp)
    // The pipeline's flags, sorted only where ordered, and the source's sizes where it keeps them.
    def characteristics(): Int =
      init()
      var c = 0
      if stage.ordered then c |= java.util.Spliterator.ORDERED
      if stage.sortedNatural && stage.ordered then c |= java.util.Spliterator.SORTED
      if stage.distinctElements then c |= java.util.Spliterator.DISTINCT
      if stage.sized then c |= sp.characteristics() & (java.util.Spliterator.SIZED | java.util.Spliterator.SUBSIZED)
      c
    // In natural order, where it is sorted.
    override def getComparator(): java.util.Comparator[? >: Any] =
      if !hasCharacteristics(java.util.Spliterator.SORTED) then throw new IllegalStateException()
      null

  // A primitive stream's spliterator over the stages' boxed elements.
  private[java] final class IntSpliterator(sp: java.util.Spliterator[Any]) extends java.util.Spliterator.OfInt:
    def tryAdvance(action: java.util.function.IntConsumer): Boolean =
      if action == null then throw new NullPointerException()
      sp.tryAdvance(t => action.accept(t.asInstanceOf[Int]))
    override def forEachRemaining(action: java.util.function.IntConsumer): Unit =
      if action == null then throw new NullPointerException()
      sp.forEachRemaining(t => action.accept(t.asInstanceOf[Int]))
    def trySplit(): java.util.Spliterator.OfInt = null
    def estimateSize(): Long = sp.estimateSize()
    override def getExactSizeIfKnown(): Long = sp.getExactSizeIfKnown()
    def characteristics(): Int = sp.characteristics()
    override def getComparator(): java.util.Comparator[? >: java.lang.Integer] = sp.getComparator()

  private[java] final class LongSpliterator(sp: java.util.Spliterator[Any]) extends java.util.Spliterator.OfLong:
    def tryAdvance(action: java.util.function.LongConsumer): Boolean =
      if action == null then throw new NullPointerException()
      sp.tryAdvance(t => action.accept(t.asInstanceOf[Long]))
    override def forEachRemaining(action: java.util.function.LongConsumer): Unit =
      if action == null then throw new NullPointerException()
      sp.forEachRemaining(t => action.accept(t.asInstanceOf[Long]))
    def trySplit(): java.util.Spliterator.OfLong = null
    def estimateSize(): Long = sp.estimateSize()
    override def getExactSizeIfKnown(): Long = sp.getExactSizeIfKnown()
    def characteristics(): Int = sp.characteristics()
    override def getComparator(): java.util.Comparator[? >: java.lang.Long] = sp.getComparator()

  private[java] final class DoubleSpliterator(sp: java.util.Spliterator[Any]) extends java.util.Spliterator.OfDouble:
    def tryAdvance(action: java.util.function.DoubleConsumer): Boolean =
      if action == null then throw new NullPointerException()
      sp.tryAdvance(t => action.accept(t.asInstanceOf[Double]))
    override def forEachRemaining(action: java.util.function.DoubleConsumer): Unit =
      if action == null then throw new NullPointerException()
      sp.forEachRemaining(t => action.accept(t.asInstanceOf[Double]))
    def trySplit(): java.util.Spliterator.OfDouble = null
    def estimateSize(): Long = sp.estimateSize()
    override def getExactSizeIfKnown(): Long = sp.getExactSizeIfKnown()
    def characteristics(): Int = sp.characteristics()
    override def getComparator(): java.util.Comparator[? >: java.lang.Double] = sp.getComparator()

  // A source stage's spliterator from a supplier, asked for at the first use.
  private[java] final class LazySpliterator(supply: java.util.function.Supplier[java.util.Spliterator[Any]]) extends java.util.Spliterator[Any]:
    private var sp: java.util.Spliterator[Any] = null
    private def get(): java.util.Spliterator[Any] =
      if sp == null then sp = supply.get()
      sp
    def tryAdvance(action: java.util.function.Consumer[? >: Any]): Boolean = get().tryAdvance(action)
    override def forEachRemaining(action: java.util.function.Consumer[? >: Any]): Unit = get().forEachRemaining(action)
    def trySplit(): java.util.Spliterator[Any] = null
    def estimateSize(): Long = get().estimateSize()
    def characteristics(): Int = get().characteristics()
    override def getComparator(): java.util.Comparator[? >: Any] = get().getComparator()

  // `Streams.ConcatSpliterator`: the first's elements, then the second's; sized where both are.
  private[java] final class Concatenation[T](a: java.util.Spliterator[T], b: java.util.Spliterator[T]) extends java.util.Spliterator[T]:
    private var beforeSplit = true
    private val unsized = a.estimateSize() + b.estimateSize() < 0
    def tryAdvance(action: java.util.function.Consumer[? >: T]): Boolean =
      if beforeSplit then
        if a.tryAdvance(action) then true
        else
          beforeSplit = false
          b.tryAdvance(action)
      else b.tryAdvance(action)
    override def forEachRemaining(action: java.util.function.Consumer[? >: T]): Unit =
      if beforeSplit then a.forEachRemaining(action)
      b.forEachRemaining(action)
    def trySplit(): java.util.Spliterator[T] = null
    def estimateSize(): Long =
      if beforeSplit then
        val size = a.estimateSize() + b.estimateSize()
        if size >= 0 then size else Long.MaxValue
      else b.estimateSize()
    def characteristics(): Int =
      if beforeSplit then
        val sizes = if unsized then java.util.Spliterator.SIZED | java.util.Spliterator.SUBSIZED else 0
        a.characteristics() & b.characteristics() & ~(java.util.Spliterator.DISTINCT | java.util.Spliterator.SORTED | sizes)
      else b.characteristics()
    // The second's, once the first is done; before, the two are not sorted together.
    override def getComparator(): java.util.Comparator[? >: T] =
      if beforeSplit then throw new IllegalStateException() else b.getComparator()

  // `Stream.iterate(seed, f)`: the seed, then `f` of the element before, without end.
  private final class Iterating[T](seed: T, f: java.util.function.UnaryOperator[T]) extends java.util.Spliterator[T]:
    private var prev: T = null.asInstanceOf[T]
    private var started = false
    def tryAdvance(action: java.util.function.Consumer[? >: T]): Boolean =
      if action == null then throw new NullPointerException()
      val t =
        if started then f.apply(prev)
        else
          started = true
          seed
      prev = t
      action.asInstanceOf[java.util.function.Consumer[T]].accept(t)
      true
    def trySplit(): java.util.Spliterator[T] = null
    def estimateSize(): Long = Long.MaxValue
    def characteristics(): Int = java.util.Spliterator.ORDERED | java.util.Spliterator.IMMUTABLE

  // `Stream.iterate(seed, hasNext, next)`: as long as `hasNext` holds of the next element.
  private final class IteratingWhile[T](seed: T, hasNext: java.util.function.Predicate[T], next: java.util.function.UnaryOperator[T]) extends java.util.Spliterator[T]:
    private var prev: T = null.asInstanceOf[T]
    private var started = false
    private var finished = false
    def tryAdvance(action: java.util.function.Consumer[? >: T]): Boolean =
      if action == null then throw new NullPointerException()
      if finished then false
      else
        val t =
          if started then next.apply(prev)
          else
            started = true
            seed
        if !hasNext.test(t) then
          prev = null.asInstanceOf[T]
          finished = true
          false
        else
          prev = t
          action.asInstanceOf[java.util.function.Consumer[T]].accept(t)
          true
    override def forEachRemaining(action: java.util.function.Consumer[? >: T]): Unit =
      if action == null then throw new NullPointerException()
      if !finished then
        finished = true
        var t = if started then next.apply(prev) else seed
        prev = null.asInstanceOf[T]
        while hasNext.test(t) do
          action.asInstanceOf[java.util.function.Consumer[T]].accept(t)
          t = next.apply(t)
    def trySplit(): java.util.Spliterator[T] = null
    def estimateSize(): Long = Long.MaxValue
    def characteristics(): Int = java.util.Spliterator.ORDERED | java.util.Spliterator.IMMUTABLE

  // `Stream.generate`: the supplier's values without end, in no order.
  private final class Generating[T](s: java.util.function.Supplier[T]) extends java.util.Spliterator[T]:
    def tryAdvance(action: java.util.function.Consumer[? >: T]): Boolean =
      if action == null then throw new NullPointerException()
      action.asInstanceOf[java.util.function.Consumer[T]].accept(s.get())
      true
    def trySplit(): java.util.Spliterator[T] = null
    def estimateSize(): Long = Long.MaxValue
    def characteristics(): Int = java.util.Spliterator.IMMUTABLE

  // `Streams.StreamBuilderImpl`: elements accepted until `build`, after which it refuses both.
  private final class StreamBuilder[T] extends Stream.Builder[T]:
    private var items = new java.util.ArrayList[Any]()
    def accept(t: T): Unit =
      if items == null then throw new IllegalStateException()
      items.add(t)
    def build(): Stream[T] =
      if items == null then throw new IllegalStateException()
      val array = new Array[Any](items.size())
      var i = 0
      while i < array.length do
        array(i) = items.get(i)
        i += 1
      items = null
      Pipes.refs[T](array)

  // The list of `Stream.toList`: unmodifiable, nulls allowed, an index out of its bounds an
  // `ArrayIndexOutOfBoundsException` as an array's.
  private final class FixedList[T](items: Array[Any]) extends java.util.List[T]:
    def size(): Int = items.length
    def get(index: Int): T =
      if index < 0 || index >= items.length then
        throw new ArrayIndexOutOfBoundsException("Index " + index + " out of bounds for length " + items.length)
      items(index).asInstanceOf[T]
    override def set(index: Int, e: T): T = throw new UnsupportedOperationException()
    override def add(e: T): Boolean = throw new UnsupportedOperationException()
    override def add(index: Int, e: T): Unit = throw new UnsupportedOperationException()
    override def remove(index: Int): T = throw new UnsupportedOperationException()
    override def remove(o: Any): Boolean = throw new UnsupportedOperationException()
    override def clear(): Unit = throw new UnsupportedOperationException()
