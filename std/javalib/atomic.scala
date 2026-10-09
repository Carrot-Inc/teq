// `java.util.concurrent.atomic` for JavaScript, which has one thread: an atomic is a plain cell
// with the JDK's names; on the JVM the classes are the JDK's own (`@jvmClass`). zio's `Chunk`
// builds on `AtomicInteger`.
package java.util.concurrent.atomic:

  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicInteger")
  class AtomicInteger(private var value: Int) extends java.lang.Number:
    def this() = this(0)
    def get(): Int = value
    def set(v: Int): Unit = value = v
    def lazySet(v: Int): Unit = value = v
    def getAndSet(v: Int): Int =
      val old = value
      value = v
      old
    def compareAndSet(expect: Int, update: Int): scala.Boolean =
      if value == expect then
        value = update
        true
      else false
    def weakCompareAndSet(expect: Int, update: Int): scala.Boolean = compareAndSet(expect, update)
    def getAndIncrement(): Int = getAndAdd(1)
    def getAndDecrement(): Int = getAndAdd(-1)
    def getAndAdd(delta: Int): Int =
      val old = value
      value = old + delta
      old
    def incrementAndGet(): Int = addAndGet(1)
    def decrementAndGet(): Int = addAndGet(-1)
    def addAndGet(delta: Int): Int =
      value = value + delta
      value
    def getAndUpdate(f: java.util.function.IntUnaryOperator): Int =
      val old = value
      value = f.applyAsInt(old)
      old
    def updateAndGet(f: java.util.function.IntUnaryOperator): Int =
      value = f.applyAsInt(value)
      value
    def intValue: Int = value
    def longValue: Long = value.toLong
    def floatValue: Float = value.toFloat
    def doubleValue: Double = value.toDouble
    override def toString: String = value.toString

  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicLong")
  class AtomicLong(private var value: Long) extends java.lang.Number:
    def this() = this(0L)
    def get(): Long = value
    def set(v: Long): Unit = value = v
    def lazySet(v: Long): Unit = value = v
    def getAndSet(v: Long): Long =
      val old = value
      value = v
      old
    def compareAndSet(expect: Long, update: Long): scala.Boolean =
      if value == expect then
        value = update
        true
      else false
    def weakCompareAndSet(expect: Long, update: Long): scala.Boolean = compareAndSet(expect, update)
    def getAndIncrement(): Long = getAndAdd(1L)
    def getAndDecrement(): Long = getAndAdd(-1L)
    def getAndAdd(delta: Long): Long =
      val old = value
      value = old + delta
      old
    def incrementAndGet(): Long = addAndGet(1L)
    def decrementAndGet(): Long = addAndGet(-1L)
    def addAndGet(delta: Long): Long =
      value = value + delta
      value
    def getAndUpdate(f: java.util.function.LongUnaryOperator): Long =
      val old = value
      value = f.applyAsLong(old)
      old
    def updateAndGet(f: java.util.function.LongUnaryOperator): Long =
      value = f.applyAsLong(value)
      value
    def intValue: Int = value.toInt
    def longValue: Long = value
    def floatValue: Float = value.toFloat
    def doubleValue: Double = value.toDouble
    override def toString: String = value.toString

  @javaDefined
  @jvmClass("java/util/concurrent/atomic/LongAdder")
  class LongAdder extends java.lang.Number:
    private var value: Long = 0L
    def add(x: Long): Unit = value = value + x
    def increment(): Unit = add(1L)
    def decrement(): Unit = add(-1L)
    def sum(): Long = value
    def reset(): Unit = value = 0L
    def sumThenReset(): Long =
      val s = value
      value = 0L
      s
    def intValue: Int = value.toInt
    def longValue: Long = value
    def floatValue: Float = value.toFloat
    def doubleValue: Double = value.toDouble
    override def toString: String = value.toString

  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicBoolean")
  class AtomicBoolean(private var value: scala.Boolean):
    def this() = this(false)
    def get(): scala.Boolean = value
    def set(v: scala.Boolean): Unit = value = v
    def lazySet(v: scala.Boolean): Unit = value = v
    def getAndSet(v: scala.Boolean): scala.Boolean =
      val old = value
      value = v
      old
    def compareAndSet(expect: scala.Boolean, update: scala.Boolean): scala.Boolean =
      if value == expect then
        value = update
        true
      else false
    def weakCompareAndSet(expect: scala.Boolean, update: scala.Boolean): scala.Boolean = compareAndSet(expect, update)
    override def toString: String = value.toString

  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicReference")
  class AtomicReference[V](private var value: V):
    def this() = this(null.asInstanceOf[V])
    def get(): V = value
    def set(v: V): Unit = value = v
    def lazySet(v: V): Unit = value = v
    def getAndSet(v: V): V =
      val old = value
      value = v
      old
    def compareAndSet(expect: V, update: V): scala.Boolean =
      if value.asInstanceOf[AnyRef] eq expect.asInstanceOf[AnyRef] then
        value = update
        true
      else false
    def weakCompareAndSet(expect: V, update: V): scala.Boolean = compareAndSet(expect, update)
    def getAndUpdate(f: java.util.function.UnaryOperator[V]): V =
      val old = value
      value = f.apply(old)
      old
    def updateAndGet(f: java.util.function.UnaryOperator[V]): V =
      value = f.apply(value)
      value
    override def toString: String = String.valueOf(value)

package java.util.function:
  @jvmClass("java/util/function/IntUnaryOperator")
  trait IntUnaryOperator:
    def applyAsInt(operand: Int): Int

  @jvmClass("java/util/function/LongUnaryOperator")
  trait LongUnaryOperator:
    def applyAsLong(operand: Long): Long

  @jvmClass("java/util/function/UnaryOperator")
  trait UnaryOperator[T]:
    def apply(t: T): T

  @jvmClass("java/util/function/Function")
  trait Function[T, R]:
    def apply(t: T): R

  @jvmClass("java/util/function/Function")
  object Function:
    def identity[T](): Function[T, T] = t => t

  @jvmClass("java/util/function/BiFunction")
  trait BiFunction[T, U, R]:
    def apply(t: T, u: U): R

  @jvmClass("java/util/function/Consumer")
  trait Consumer[T]:
    def accept(t: T): Unit

  @jvmClass("java/util/function/Predicate")
  trait Predicate[T]:
    def test(t: T): scala.Boolean
    // The JDK's default methods.
    def and(other: Predicate[? >: T]): Predicate[T] =
      java.util.Objects.requireNonNull(other)
      t => test(t) && other.test(t)
    def negate(): Predicate[T] = t => !test(t)
    def or(other: Predicate[? >: T]): Predicate[T] =
      java.util.Objects.requireNonNull(other)
      t => test(t) || other.test(t)

  @jvmClass("java/util/function/Predicate")
  object Predicate:
    def isEqual[T](targetRef: Any): Predicate[T] =
      if targetRef == null then t => t == null else t => targetRef.equals(t)
    def not[T](target: Predicate[? >: T]): Predicate[T] =
      java.util.Objects.requireNonNull(target)
      target.negate().asInstanceOf[Predicate[T]]

  @jvmClass("java/util/function/IntFunction")
  trait IntFunction[R]:
    def apply(value: Int): R

  @jvmClass("java/util/function/Supplier")
  trait Supplier[T]:
    def get(): T
