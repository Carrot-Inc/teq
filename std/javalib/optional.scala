// `java.util.Optional` and the optionals of the primitive streams, as the JDK's: `of` refuses null,
// `get` and `orElseThrow` of an empty one are a `NoSuchElementException` "No value present", a
// null function is refused before it is needed, and `toString`, `equals` and `hashCode` are the
// JDK's. On the JVM the classes are the JDK's.
package java.util:

  @jvmClass("java/util/Optional")
  final class Optional[T] private (private val value: T):
    def get(): T =
      if value == null then throw new NoSuchElementException("No value present")
      value
    def isPresent(): Boolean = value != null
    def isEmpty(): Boolean = value == null
    def ifPresent(action: java.util.function.Consumer[? >: T]): Unit =
      if value != null then action.asInstanceOf[java.util.function.Consumer[T]].accept(value)
    def ifPresentOrElse(action: java.util.function.Consumer[? >: T], emptyAction: Runnable): Unit =
      if value != null then action.asInstanceOf[java.util.function.Consumer[T]].accept(value)
      else emptyAction.run()
    def filter(predicate: java.util.function.Predicate[? >: T]): Optional[T] =
      if predicate == null then throw new NullPointerException()
      if value == null then this
      else if predicate.asInstanceOf[java.util.function.Predicate[T]].test(value) then this
      else Optional.empty[T]()
    def map[U](mapper: java.util.function.Function[? >: T, ? <: U]): Optional[U] =
      if mapper == null then throw new NullPointerException()
      if value == null then Optional.empty[U]()
      else Optional.ofNullable[U](mapper.asInstanceOf[java.util.function.Function[T, U]].apply(value))
    def flatMap[U](mapper: java.util.function.Function[? >: T, ? <: Optional[? <: U]]): Optional[U] =
      if mapper == null then throw new NullPointerException()
      if value == null then Optional.empty[U]()
      else
        val r = mapper.asInstanceOf[java.util.function.Function[T, Optional[U]]].apply(value)
        if r == null then throw new NullPointerException()
        r
    def or(supplier: java.util.function.Supplier[? <: Optional[? <: T]]): Optional[T] =
      if supplier == null then throw new NullPointerException()
      if value != null then this
      else
        val r = supplier.get().asInstanceOf[Optional[T]]
        if r == null then throw new NullPointerException()
        r
    def stream(): java.util.stream.Stream[T] =
      if value == null then java.util.stream.Stream.empty[T]() else java.util.stream.Stream.of[T](value)
    def orElse(other: T): T = if value != null then value else other
    def orElseGet(supplier: java.util.function.Supplier[? <: T]): T = if value != null then value else supplier.get()
    def orElseThrow(): T =
      if value == null then throw new NoSuchElementException("No value present")
      value
    def orElseThrow[X <: Throwable](exceptionSupplier: java.util.function.Supplier[? <: X]): T =
      if value != null then value else throw exceptionSupplier.get()
    override def equals(obj: Any): Boolean = obj match
      case other: Optional[?] => (this eq other) || Objects.equals(value, other.value)
      case _ => false
    override def hashCode: Int = Objects.hashCode(value)
    override def toString: String = if value != null then "Optional[" + value + "]" else "Optional.empty"

  @jvmClass("java/util/Optional")
  object Optional:
    private val EMPTY: Optional[Any] = new Optional[Any](null)
    def empty[T](): Optional[T] = EMPTY.asInstanceOf[Optional[T]]
    def of[T](value: T): Optional[T] =
      if value == null then throw new NullPointerException()
      new Optional[T](value)
    def ofNullable[T](value: T): Optional[T] = if value == null then empty[T]() else new Optional[T](value)

  @jvmClass("java/util/OptionalInt")
  final class OptionalInt private (present: Boolean, value: Int):
    def getAsInt(): Int =
      if !present then throw new NoSuchElementException("No value present")
      value
    def isPresent(): Boolean = present
    def isEmpty(): Boolean = !present
    def ifPresent(action: java.util.function.IntConsumer): Unit = if present then action.accept(value)
    def ifPresentOrElse(action: java.util.function.IntConsumer, emptyAction: Runnable): Unit =
      if present then action.accept(value) else emptyAction.run()
    def stream(): java.util.stream.IntStream =
      if present then java.util.stream.IntStream.of(value) else java.util.stream.IntStream.empty()
    def orElse(other: Int): Int = if present then value else other
    def orElseGet(supplier: java.util.function.IntSupplier): Int = if present then value else supplier.getAsInt()
    def orElseThrow(): Int =
      if !present then throw new NoSuchElementException("No value present")
      value
    def orElseThrow[X <: Throwable](exceptionSupplier: java.util.function.Supplier[? <: X]): Int =
      if present then value else throw exceptionSupplier.get()
    override def equals(obj: Any): Boolean = obj match
      case other: OptionalInt =>
        (this eq other) || (if present && other.isPresent() then value == other.getAsInt() else present == other.isPresent())
      case _ => false
    override def hashCode: Int = if present then java.lang.Integer.hashCode(value) else 0
    override def toString: String = if present then "OptionalInt[" + value + "]" else "OptionalInt.empty"

  @jvmClass("java/util/OptionalInt")
  object OptionalInt:
    private val EMPTY = new OptionalInt(false, 0)
    def empty(): OptionalInt = EMPTY
    def of(value: Int): OptionalInt = new OptionalInt(true, value)

  @jvmClass("java/util/OptionalLong")
  final class OptionalLong private (present: Boolean, value: Long):
    def getAsLong(): Long =
      if !present then throw new NoSuchElementException("No value present")
      value
    def isPresent(): Boolean = present
    def isEmpty(): Boolean = !present
    def ifPresent(action: java.util.function.LongConsumer): Unit = if present then action.accept(value)
    def ifPresentOrElse(action: java.util.function.LongConsumer, emptyAction: Runnable): Unit =
      if present then action.accept(value) else emptyAction.run()
    def stream(): java.util.stream.LongStream =
      if present then java.util.stream.LongStream.of(value) else java.util.stream.LongStream.empty()
    def orElse(other: Long): Long = if present then value else other
    def orElseGet(supplier: java.util.function.LongSupplier): Long = if present then value else supplier.getAsLong()
    def orElseThrow(): Long =
      if !present then throw new NoSuchElementException("No value present")
      value
    def orElseThrow[X <: Throwable](exceptionSupplier: java.util.function.Supplier[? <: X]): Long =
      if present then value else throw exceptionSupplier.get()
    override def equals(obj: Any): Boolean = obj match
      case other: OptionalLong =>
        (this eq other) || (if present && other.isPresent() then value == other.getAsLong() else present == other.isPresent())
      case _ => false
    override def hashCode: Int = if present then java.lang.Long.hashCode(value) else 0
    override def toString: String = if present then "OptionalLong[" + value + "]" else "OptionalLong.empty"

  @jvmClass("java/util/OptionalLong")
  object OptionalLong:
    private val EMPTY = new OptionalLong(false, 0L)
    def empty(): OptionalLong = EMPTY
    def of(value: Long): OptionalLong = new OptionalLong(true, value)

  @jvmClass("java/util/OptionalDouble")
  final class OptionalDouble private (present: Boolean, value: Double):
    def getAsDouble(): Double =
      if !present then throw new NoSuchElementException("No value present")
      value
    def isPresent(): Boolean = present
    def isEmpty(): Boolean = !present
    def ifPresent(action: java.util.function.DoubleConsumer): Unit = if present then action.accept(value)
    def ifPresentOrElse(action: java.util.function.DoubleConsumer, emptyAction: Runnable): Unit =
      if present then action.accept(value) else emptyAction.run()
    def stream(): java.util.stream.DoubleStream =
      if present then java.util.stream.DoubleStream.of(value) else java.util.stream.DoubleStream.empty()
    def orElse(other: Double): Double = if present then value else other
    def orElseGet(supplier: java.util.function.DoubleSupplier): Double = if present then value else supplier.getAsDouble()
    def orElseThrow(): Double =
      if !present then throw new NoSuchElementException("No value present")
      value
    def orElseThrow[X <: Throwable](exceptionSupplier: java.util.function.Supplier[? <: X]): Double =
      if present then value else throw exceptionSupplier.get()
    // Two present values are equal as `Double.compare` has them: NaN equals NaN, 0.0 not -0.0.
    override def equals(obj: Any): Boolean = obj match
      case other: OptionalDouble =>
        (this eq other) || (if present && other.isPresent() then java.lang.Double.compare(value, other.getAsDouble()) == 0 else present == other.isPresent())
      case _ => false
    override def hashCode: Int = if present then java.lang.Double.hashCode(value) else 0
    override def toString: String = if present then "OptionalDouble[" + value + "]" else "OptionalDouble.empty"

  @jvmClass("java/util/OptionalDouble")
  object OptionalDouble:
    private val EMPTY = new OptionalDouble(false, 0.0)
    def empty(): OptionalDouble = EMPTY
    def of(value: Double): OptionalDouble = new OptionalDouble(true, value)
