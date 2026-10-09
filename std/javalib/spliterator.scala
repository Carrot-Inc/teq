// What a stream is made from and the functional interfaces of its operations, as the JDK's:
// `Spliterator` and the `Spliterators` over an iterator or a collection, the primitive iterators,
// `StringJoiner`, and the interfaces of `java.util.function` the stream and optional classes take
// beyond the ones the std already has. Sequential only: `trySplit` splits nothing. On the JVM the
// classes are the JDK's.
package java.util:

  @jvmClass("java/util/Spliterator")
  trait Spliterator[T]:
    def tryAdvance(action: java.util.function.Consumer[? >: T]): Boolean
    def forEachRemaining(action: java.util.function.Consumer[? >: T]): Unit =
      while tryAdvance(action) do ()
    def trySplit(): Spliterator[T]
    def estimateSize(): Long
    def getExactSizeIfKnown(): Long = if (characteristics() & Spliterator.SIZED) == 0 then -1L else estimateSize()
    def characteristics(): Int
    def hasCharacteristics(characteristics: Int): Boolean = (this.characteristics() & characteristics) == characteristics
    def getComparator(): Comparator[? >: T] = throw new IllegalStateException()

  @jvmClass("java/util/Spliterator")
  object Spliterator:
    final val ORDERED = 0x00000010
    final val DISTINCT = 0x00000001
    final val SORTED = 0x00000004
    final val SIZED = 0x00000040
    final val NONNULL = 0x00000100
    final val IMMUTABLE = 0x00000400
    final val CONCURRENT = 0x00001000
    final val SUBSIZED = 0x00004000

    @jvmClass("java/util/Spliterator$OfPrimitive")
    trait OfPrimitive[T, T_CONS, T_SPLITR <: OfPrimitive[T, T_CONS, T_SPLITR]] extends Spliterator[T]:
      override def trySplit(): T_SPLITR
      def tryAdvance(action: T_CONS): Boolean
      def forEachRemaining(action: T_CONS): Unit =
        while tryAdvance(action) do ()

    // A primitive spliterator taking a `Consumer` hands it the boxes.
    @jvmClass("java/util/Spliterator$OfInt")
    trait OfInt extends OfPrimitive[java.lang.Integer, java.util.function.IntConsumer, OfInt]:
      override def trySplit(): OfInt
      def tryAdvance(action: java.util.function.IntConsumer): Boolean
      override def forEachRemaining(action: java.util.function.IntConsumer): Unit =
        while tryAdvance(action) do ()
      override def tryAdvance(action: java.util.function.Consumer[? >: java.lang.Integer]): Boolean =
        if action == null then throw new NullPointerException()
        val c = action.asInstanceOf[java.util.function.Consumer[Any]]
        tryAdvance(Spliterator.intConsumer(c))
      override def forEachRemaining(action: java.util.function.Consumer[? >: java.lang.Integer]): Unit =
        if action == null then throw new NullPointerException()
        val c = action.asInstanceOf[java.util.function.Consumer[Any]]
        forEachRemaining(Spliterator.intConsumer(c))

    @jvmClass("java/util/Spliterator$OfLong")
    trait OfLong extends OfPrimitive[java.lang.Long, java.util.function.LongConsumer, OfLong]:
      override def trySplit(): OfLong
      def tryAdvance(action: java.util.function.LongConsumer): Boolean
      override def forEachRemaining(action: java.util.function.LongConsumer): Unit =
        while tryAdvance(action) do ()
      override def tryAdvance(action: java.util.function.Consumer[? >: java.lang.Long]): Boolean =
        if action == null then throw new NullPointerException()
        val c = action.asInstanceOf[java.util.function.Consumer[Any]]
        tryAdvance(Spliterator.longConsumer(c))
      override def forEachRemaining(action: java.util.function.Consumer[? >: java.lang.Long]): Unit =
        if action == null then throw new NullPointerException()
        val c = action.asInstanceOf[java.util.function.Consumer[Any]]
        forEachRemaining(Spliterator.longConsumer(c))

    @jvmClass("java/util/Spliterator$OfDouble")
    trait OfDouble extends OfPrimitive[java.lang.Double, java.util.function.DoubleConsumer, OfDouble]:
      override def trySplit(): OfDouble
      def tryAdvance(action: java.util.function.DoubleConsumer): Boolean
      override def forEachRemaining(action: java.util.function.DoubleConsumer): Unit =
        while tryAdvance(action) do ()
      override def tryAdvance(action: java.util.function.Consumer[? >: java.lang.Double]): Boolean =
        if action == null then throw new NullPointerException()
        val c = action.asInstanceOf[java.util.function.Consumer[Any]]
        tryAdvance(Spliterator.doubleConsumer(c))
      override def forEachRemaining(action: java.util.function.Consumer[? >: java.lang.Double]): Unit =
        if action == null then throw new NullPointerException()
        val c = action.asInstanceOf[java.util.function.Consumer[Any]]
        forEachRemaining(Spliterator.doubleConsumer(c))

    private def intConsumer(c: java.util.function.Consumer[Any]): java.util.function.IntConsumer = v => c.accept(v)
    private def longConsumer(c: java.util.function.Consumer[Any]): java.util.function.LongConsumer = v => c.accept(v)
    private def doubleConsumer(c: java.util.function.Consumer[Any]): java.util.function.DoubleConsumer = v => c.accept(v)

  @jvmClass("java/util/Spliterators")
  object Spliterators:
    def emptySpliterator[T](): Spliterator[T] = new ArraySpliterator[T](new Array[Any](0), Spliterator.SIZED | Spliterator.SUBSIZED)
    // Over a collection: its iterator and size taken when the traversal starts (late-binding).
    def spliterator[T](c: Collection[? <: T], characteristics: Int): Spliterator[T] =
      if c == null then throw new NullPointerException()
      new IteratorSpliterator[T](c.asInstanceOf[Collection[T]], null, Long.MaxValue, sizedUnlessConcurrent(characteristics))
    def spliterator[T](iterator: Iterator[? <: T], size: Long, characteristics: Int): Spliterator[T] =
      if iterator == null then throw new NullPointerException()
      new IteratorSpliterator[T](null, iterator.asInstanceOf[Iterator[T]], size, sizedUnlessConcurrent(characteristics))
    def spliteratorUnknownSize[T](iterator: Iterator[? <: T], characteristics: Int): Spliterator[T] =
      if iterator == null then throw new NullPointerException()
      new IteratorSpliterator[T](null, iterator.asInstanceOf[Iterator[T]], Long.MaxValue, characteristics & ~(Spliterator.SIZED | Spliterator.SUBSIZED))
    // The JDK's adapter: `hasNext` takes the next element ahead, which `next` gives even once the
    // stream is closed.
    def iterator[T](spliterator: Spliterator[? <: T]): Iterator[T] =
      if spliterator == null then throw new NullPointerException()
      new SpliteratorIterator[T](spliterator.asInstanceOf[Spliterator[T]])
    private def sizedUnlessConcurrent(characteristics: Int): Int =
      if (characteristics & Spliterator.CONCURRENT) == 0 then characteristics | Spliterator.SIZED | Spliterator.SUBSIZED
      else characteristics

  // The elements of an array from an index on: what `Stream.of` and a builder's stream traverse.
  private[java] final class ArraySpliterator[T](items: Array[Any], flags: Int) extends Spliterator[T]:
    private var index = 0
    def tryAdvance(action: java.util.function.Consumer[? >: T]): Boolean =
      if action == null then throw new NullPointerException()
      if index < items.length then
        index += 1
        action.asInstanceOf[java.util.function.Consumer[Any]].accept(items(index - 1))
        true
      else false
    override def forEachRemaining(action: java.util.function.Consumer[? >: T]): Unit =
      if action == null then throw new NullPointerException()
      val from = index
      index = items.length
      var i = from
      while i < items.length do
        action.asInstanceOf[java.util.function.Consumer[Any]].accept(items(i))
        i += 1
    def trySplit(): Spliterator[T] = null
    def estimateSize(): Long = items.length - index
    def characteristics(): Int = flags
    // Sorted, it is in natural order.
    override def getComparator(): Comparator[? >: T] =
      if hasCharacteristics(Spliterator.SORTED) then null else throw new IllegalStateException()

  // `Spliterators.IteratorSpliterator`: an iterator, or a collection's taken at the first use.
  private final class IteratorSpliterator[T](collection: Collection[T], private var it: Iterator[T], private var est: Long, flags: Int) extends Spliterator[T]:
    private def bind(): Iterator[T] =
      if it == null then
        it = collection.iterator()
        est = collection.size()
      it
    def tryAdvance(action: java.util.function.Consumer[? >: T]): Boolean =
      if action == null then throw new NullPointerException()
      val i = bind()
      if i.hasNext then
        action.asInstanceOf[java.util.function.Consumer[T]].accept(i.next())
        true
      else false
    override def forEachRemaining(action: java.util.function.Consumer[? >: T]): Unit =
      if action == null then throw new NullPointerException()
      val i = bind()
      while i.hasNext do action.asInstanceOf[java.util.function.Consumer[T]].accept(i.next())
    def trySplit(): Spliterator[T] = null
    def estimateSize(): Long =
      if it == null then
        it = collection.iterator()
        est = collection.size()
      est
    def characteristics(): Int = flags
    // Sorted, it is in natural order.
    override def getComparator(): Comparator[? >: T] =
      if hasCharacteristics(Spliterator.SORTED) then null else throw new IllegalStateException()

  private final class SpliteratorIterator[T](spliterator: Spliterator[T]) extends Iterator[T] with java.util.function.Consumer[T]:
    private var ready = false
    private var element: T = null.asInstanceOf[T]
    def accept(t: T): Unit =
      ready = true
      element = t
    def hasNext: Boolean =
      if !ready then spliterator.tryAdvance(this)
      ready
    def next(): T =
      if !ready && !hasNext then throw new NoSuchElementException()
      ready = false
      val out = element
      element = null.asInstanceOf[T]
      out

  @jvmClass("java/util/PrimitiveIterator")
  trait PrimitiveIterator[T, T_CONS] extends Iterator[T]:
    def forEachRemaining(action: T_CONS): Unit

  @jvmClass("java/util/PrimitiveIterator")
  object PrimitiveIterator:
    @jvmClass("java/util/PrimitiveIterator$OfInt")
    trait OfInt extends PrimitiveIterator[java.lang.Integer, java.util.function.IntConsumer]:
      def nextInt(): Int
      override def next(): java.lang.Integer = java.lang.Integer.valueOf(nextInt())
      def forEachRemaining(action: java.util.function.IntConsumer): Unit =
        if action == null then throw new NullPointerException()
        while hasNext do action.accept(nextInt())
    @jvmClass("java/util/PrimitiveIterator$OfLong")
    trait OfLong extends PrimitiveIterator[java.lang.Long, java.util.function.LongConsumer]:
      def nextLong(): Long
      override def next(): java.lang.Long = java.lang.Long.valueOf(nextLong())
      def forEachRemaining(action: java.util.function.LongConsumer): Unit =
        if action == null then throw new NullPointerException()
        while hasNext do action.accept(nextLong())
    @jvmClass("java/util/PrimitiveIterator$OfDouble")
    trait OfDouble extends PrimitiveIterator[java.lang.Double, java.util.function.DoubleConsumer]:
      def nextDouble(): Double
      override def next(): java.lang.Double = java.lang.Double.valueOf(nextDouble())
      def forEachRemaining(action: java.util.function.DoubleConsumer): Unit =
        if action == null then throw new NullPointerException()
        while hasNext do action.accept(nextDouble())

  // The JDK's: the elements as strings, between the prefix and the suffix and apart by the
  // delimiter, or the empty value when there are none.
  @jvmClass("java/util/StringJoiner")
  final class StringJoiner(delimiter: CharSequence, prefix: CharSequence, suffix: CharSequence):
    if prefix == null then throw new NullPointerException("The prefix must not be null")
    if delimiter == null then throw new NullPointerException("The delimiter must not be null")
    if suffix == null then throw new NullPointerException("The suffix must not be null")
    private val pre = prefix.toString
    private val delim = delimiter.toString
    private val suf = suffix.toString
    private val joined = new java.lang.StringBuilder()
    private var size = 0
    private var emptyValue: String = null
    def this(delimiter: CharSequence) = this(delimiter, "", "")
    def setEmptyValue(emptyValue: CharSequence): StringJoiner =
      if emptyValue == null then throw new NullPointerException("The empty value must not be null")
      this.emptyValue = emptyValue.toString
      this
    override def toString: String =
      if size == 0 && emptyValue != null then emptyValue
      else pre + joined.toString + suf
    def add(newElement: CharSequence): StringJoiner =
      val elt = String.valueOf(newElement)
      if size > 0 then joined.append(delim)
      joined.append(elt)
      size += 1
      this
    // The other's elements, joined by its delimiter, as one element of this one.
    def merge(other: StringJoiner): StringJoiner =
      if other == null then throw new NullPointerException()
      if other.size == 0 then this else add(other.joined.toString)
    def length(): Int =
      if size == 0 && emptyValue != null then emptyValue.length
      else joined.length() + pre.length + suf.length

package java.util.function:
  @jvmClass("java/util/function/BiConsumer")
  trait BiConsumer[T, U]:
    def accept(t: T, u: U): Unit

  @jvmClass("java/util/function/BinaryOperator")
  trait BinaryOperator[T] extends BiFunction[T, T, T]

  @jvmClass("java/util/function/BinaryOperator")
  object BinaryOperator:
    def minBy[T](comparator: java.util.Comparator[? >: T]): BinaryOperator[T] =
      if comparator == null then throw new NullPointerException()
      val c = comparator.asInstanceOf[java.util.Comparator[T]]
      (a, b) => if c.compare(a, b) <= 0 then a else b
    def maxBy[T](comparator: java.util.Comparator[? >: T]): BinaryOperator[T] =
      if comparator == null then throw new NullPointerException()
      val c = comparator.asInstanceOf[java.util.Comparator[T]]
      (a, b) => if c.compare(a, b) >= 0 then a else b

  @jvmClass("java/util/function/ToIntFunction")
  trait ToIntFunction[T]:
    def applyAsInt(value: T): Int

  @jvmClass("java/util/function/ToLongFunction")
  trait ToLongFunction[T]:
    def applyAsLong(value: T): Long

  @jvmClass("java/util/function/ToDoubleFunction")
  trait ToDoubleFunction[T]:
    def applyAsDouble(value: T): Double

  @jvmClass("java/util/function/IntConsumer")
  trait IntConsumer:
    def accept(value: Int): Unit

  @jvmClass("java/util/function/LongConsumer")
  trait LongConsumer:
    def accept(value: Long): Unit

  @jvmClass("java/util/function/DoubleConsumer")
  trait DoubleConsumer:
    def accept(value: Double): Unit

  @jvmClass("java/util/function/IntSupplier")
  trait IntSupplier:
    def getAsInt(): Int

  @jvmClass("java/util/function/LongSupplier")
  trait LongSupplier:
    def getAsLong(): Long

  @jvmClass("java/util/function/DoubleSupplier")
  trait DoubleSupplier:
    def getAsDouble(): Double
