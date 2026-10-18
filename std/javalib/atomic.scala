// `java.util.concurrent.atomic`: on JavaScript, which has one thread, an atomic is a plain cell with the
// JDK's names; on the JVM the classes are the JDK's own (`@jvmClass`). Every read-modify-write goes
// through the class's `compareAndSet`, `getAndSet` and `getAndAdd`, as the JDK's go through its
// `Unsafe`, so that under `teq interp`, whose logical threads may switch between two steps of a body,
// they are the natives that make each one indivisible (src/interp/natives.rs, `atomics`): a native runs
// whole. zio's `Chunk` builds on `AtomicInteger`.
package java.util.concurrent.atomic:

  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicInteger")
  class AtomicInteger(private var value: Int) extends java.lang.Number:
    def this() = this(0)
    def get(): Int = value
    def set(v: Int): Unit = value = v
    def lazySet(v: Int): Unit = value = v
    def getPlain(): Int = value
    def setPlain(v: Int): Unit = value = v
    def getAcquire(): Int = value
    def setRelease(v: Int): Unit = value = v
    def getOpaque(): Int = value
    def setOpaque(v: Int): Unit = value = v
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
    def weakCompareAndSetVolatile(expect: Int, update: Int): scala.Boolean = compareAndSet(expect, update)
    def weakCompareAndSetPlain(expect: Int, update: Int): scala.Boolean = compareAndSet(expect, update)
    def compareAndExchange(expect: Int, update: Int): Int =
      var witness = value
      while witness == expect && !compareAndSet(expect, update) do witness = value
      witness
    def getAndAdd(delta: Int): Int =
      val old = value
      value = old + delta
      old
    def getAndIncrement(): Int = getAndAdd(1)
    def getAndDecrement(): Int = getAndAdd(-1)
    def incrementAndGet(): Int = getAndAdd(1) + 1
    def decrementAndGet(): Int = getAndAdd(-1) - 1
    def addAndGet(delta: Int): Int = getAndAdd(delta) + delta
    def getAndUpdate(f: java.util.function.IntUnaryOperator): Int =
      var prev = get()
      var next = f.applyAsInt(prev)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.applyAsInt(prev)
      prev
    def updateAndGet(f: java.util.function.IntUnaryOperator): Int =
      var prev = get()
      var next = f.applyAsInt(prev)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.applyAsInt(prev)
      next
    def getAndAccumulate(x: Int, f: java.util.function.IntBinaryOperator): Int =
      var prev = get()
      var next = f.applyAsInt(prev, x)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.applyAsInt(prev, x)
      prev
    def accumulateAndGet(x: Int, f: java.util.function.IntBinaryOperator): Int =
      var prev = get()
      var next = f.applyAsInt(prev, x)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.applyAsInt(prev, x)
      next
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
    def getPlain(): Long = value
    def setPlain(v: Long): Unit = value = v
    def getAcquire(): Long = value
    def setRelease(v: Long): Unit = value = v
    def getOpaque(): Long = value
    def setOpaque(v: Long): Unit = value = v
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
    def weakCompareAndSetVolatile(expect: Long, update: Long): scala.Boolean = compareAndSet(expect, update)
    def weakCompareAndSetPlain(expect: Long, update: Long): scala.Boolean = compareAndSet(expect, update)
    def compareAndExchange(expect: Long, update: Long): Long =
      var witness = value
      while witness == expect && !compareAndSet(expect, update) do witness = value
      witness
    def getAndAdd(delta: Long): Long =
      val old = value
      value = old + delta
      old
    def getAndIncrement(): Long = getAndAdd(1L)
    def getAndDecrement(): Long = getAndAdd(-1L)
    def incrementAndGet(): Long = getAndAdd(1L) + 1L
    def decrementAndGet(): Long = getAndAdd(-1L) - 1L
    def addAndGet(delta: Long): Long = getAndAdd(delta) + delta
    def getAndUpdate(f: java.util.function.LongUnaryOperator): Long =
      var prev = get()
      var next = f.applyAsLong(prev)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.applyAsLong(prev)
      prev
    def updateAndGet(f: java.util.function.LongUnaryOperator): Long =
      var prev = get()
      var next = f.applyAsLong(prev)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.applyAsLong(prev)
      next
    def getAndAccumulate(x: Long, f: java.util.function.LongBinaryOperator): Long =
      var prev = get()
      var next = f.applyAsLong(prev, x)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.applyAsLong(prev, x)
      prev
    def accumulateAndGet(x: Long, f: java.util.function.LongBinaryOperator): Long =
      var prev = get()
      var next = f.applyAsLong(prev, x)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.applyAsLong(prev, x)
      next
    def intValue: Int = value.toInt
    def longValue: Long = value
    def floatValue: Float = value.toFloat
    def doubleValue: Double = value.toDouble
    override def toString: String = value.toString

  // One cell here, where the JDK spreads the sum over cells against contention; `add` is the
  // indivisible one.
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
      add(-s)
      s
    def intValue: Int = sum().toInt
    def longValue: Long = sum()
    def floatValue: Float = sum().toFloat
    def doubleValue: Double = sum().toDouble
    override def toString: String = sum().toString

  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicBoolean")
  class AtomicBoolean(private var value: scala.Boolean):
    def this() = this(false)
    def get(): scala.Boolean = value
    def set(v: scala.Boolean): Unit = value = v
    def lazySet(v: scala.Boolean): Unit = value = v
    def getPlain(): scala.Boolean = value
    def setPlain(v: scala.Boolean): Unit = value = v
    def getAcquire(): scala.Boolean = value
    def setRelease(v: scala.Boolean): Unit = value = v
    def getOpaque(): scala.Boolean = value
    def setOpaque(v: scala.Boolean): Unit = value = v
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
    def weakCompareAndSetVolatile(expect: scala.Boolean, update: scala.Boolean): scala.Boolean = compareAndSet(expect, update)
    def weakCompareAndSetPlain(expect: scala.Boolean, update: scala.Boolean): scala.Boolean = compareAndSet(expect, update)
    def compareAndExchange(expect: scala.Boolean, update: scala.Boolean): scala.Boolean =
      var witness = value
      while witness == expect && !compareAndSet(expect, update) do witness = value
      witness
    override def toString: String = value.toString

  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicReference")
  class AtomicReference[V](private var value: V):
    def this() = this(null.asInstanceOf[V])
    def get(): V = value
    def set(v: V): Unit = value = v
    def lazySet(v: V): Unit = value = v
    def getPlain(): V = value
    def setPlain(v: V): Unit = value = v
    def getAcquire(): V = value
    def setRelease(v: V): Unit = value = v
    def getOpaque(): V = value
    def setOpaque(v: V): Unit = value = v
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
    def weakCompareAndSetVolatile(expect: V, update: V): scala.Boolean = compareAndSet(expect, update)
    def weakCompareAndSetPlain(expect: V, update: V): scala.Boolean = compareAndSet(expect, update)
    def compareAndExchange(expect: V, update: V): V =
      var witness = value
      while (witness.asInstanceOf[AnyRef] eq expect.asInstanceOf[AnyRef]) && !compareAndSet(expect, update) do witness = value
      witness
    def getAndUpdate(f: java.util.function.UnaryOperator[V]): V =
      var prev = get()
      var next = f.apply(prev)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.apply(prev)
      prev
    def updateAndGet(f: java.util.function.UnaryOperator[V]): V =
      var prev = get()
      var next = f.apply(prev)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.apply(prev)
      next
    def getAndAccumulate(x: V, f: java.util.function.BinaryOperator[V]): V =
      var prev = get()
      var next = f.apply(prev, x)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.apply(prev, x)
      prev
    def accumulateAndGet(x: V, f: java.util.function.BinaryOperator[V]): V =
      var prev = get()
      var next = f.apply(prev, x)
      while !compareAndSet(prev, next) do
        prev = get()
        next = f.apply(prev, x)
      next
    override def toString: String = String.valueOf(value)

  // The arrays of atomics: an array whose elements each read, write and compare-and-set as the cells
  // above (`compareAndSet` and `getAndAdd` the indivisible ones).
  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicIntegerArray")
  class AtomicIntegerArray private (private val array: Array[Int]):
    def this(length: Int) = this(new Array[Int](length))
    def this(source: Array[Int], copy: scala.Boolean) = this(source.clone())
    def length(): Int = array.length
    def get(i: Int): Int = array(i)
    def set(i: Int, v: Int): Unit = array(i) = v
    def lazySet(i: Int, v: Int): Unit = array(i) = v
    def getAndSet(i: Int, v: Int): Int =
      val old = array(i)
      array(i) = v
      old
    def compareAndSet(i: Int, expect: Int, update: Int): scala.Boolean =
      if array(i) == expect then
        array(i) = update
        true
      else false
    def weakCompareAndSet(i: Int, expect: Int, update: Int): scala.Boolean = compareAndSet(i, expect, update)
    def getAndAdd(i: Int, delta: Int): Int =
      val old = array(i)
      array(i) = old + delta
      old
    def getAndIncrement(i: Int): Int = getAndAdd(i, 1)
    def getAndDecrement(i: Int): Int = getAndAdd(i, -1)
    def incrementAndGet(i: Int): Int = getAndAdd(i, 1) + 1
    def decrementAndGet(i: Int): Int = getAndAdd(i, -1) - 1
    def addAndGet(i: Int, delta: Int): Int = getAndAdd(i, delta) + delta
    override def toString: String = java.util.Arrays.toString(array)

  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicLongArray")
  class AtomicLongArray private (private val array: Array[Long]):
    def this(length: Int) = this(new Array[Long](length))
    def length(): Int = array.length
    def get(i: Int): Long = array(i)
    def set(i: Int, v: Long): Unit = array(i) = v
    def lazySet(i: Int, v: Long): Unit = array(i) = v
    def getAndSet(i: Int, v: Long): Long =
      val old = array(i)
      array(i) = v
      old
    def compareAndSet(i: Int, expect: Long, update: Long): scala.Boolean =
      if array(i) == expect then
        array(i) = update
        true
      else false
    def weakCompareAndSet(i: Int, expect: Long, update: Long): scala.Boolean = compareAndSet(i, expect, update)
    def getAndAdd(i: Int, delta: Long): Long =
      val old = array(i)
      array(i) = old + delta
      old
    def getAndIncrement(i: Int): Long = getAndAdd(i, 1L)
    def getAndDecrement(i: Int): Long = getAndAdd(i, -1L)
    def incrementAndGet(i: Int): Long = getAndAdd(i, 1L) + 1L
    def decrementAndGet(i: Int): Long = getAndAdd(i, -1L) - 1L
    def addAndGet(i: Int, delta: Long): Long = getAndAdd(i, delta) + delta
    override def toString: String = java.util.Arrays.toString(array)

  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicReferenceArray")
  class AtomicReferenceArray[E] private (private val array: Array[AnyRef]):
    def this(length: Int) = this(new Array[AnyRef](length))
    def length(): Int = array.length
    def get(i: Int): E = array(i).asInstanceOf[E]
    def set(i: Int, v: E): Unit = array(i) = v.asInstanceOf[AnyRef]
    def lazySet(i: Int, v: E): Unit = array(i) = v.asInstanceOf[AnyRef]
    def getAndSet(i: Int, v: E): E =
      val old = array(i)
      array(i) = v.asInstanceOf[AnyRef]
      old.asInstanceOf[E]
    def compareAndSet(i: Int, expect: E, update: E): scala.Boolean =
      if array(i) eq expect.asInstanceOf[AnyRef] then
        array(i) = update.asInstanceOf[AnyRef]
        true
      else false
    def weakCompareAndSet(i: Int, expect: E, update: E): scala.Boolean = compareAndSet(i, expect, update)
    override def toString: String = java.util.Arrays.toString(array)

  // The JDK's updaters of a class's `volatile long` field by its name: under `teq interp` the field of
  // the object read and written by the interpreter (`fieldUpdate`), its reads and writes indivisible.
  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicLongFieldUpdater")
  abstract class AtomicLongFieldUpdater[T] protected ():
    def compareAndSet(obj: T, expect: Long, update: Long): scala.Boolean
    def weakCompareAndSet(obj: T, expect: Long, update: Long): scala.Boolean = compareAndSet(obj, expect, update)
    def set(obj: T, newValue: Long): Unit
    def lazySet(obj: T, newValue: Long): Unit = set(obj, newValue)
    def get(obj: T): Long
    def getAndSet(obj: T, newValue: Long): Long
    def getAndAdd(obj: T, delta: Long): Long
    def getAndIncrement(obj: T): Long = getAndAdd(obj, 1L)
    def getAndDecrement(obj: T): Long = getAndAdd(obj, -1L)
    def incrementAndGet(obj: T): Long = getAndAdd(obj, 1L) + 1L
    def decrementAndGet(obj: T): Long = getAndAdd(obj, -1L) - 1L
    def addAndGet(obj: T, delta: Long): Long = getAndAdd(obj, delta) + delta

  @javaDefined
  @jvmClass("java/util/concurrent/atomic/AtomicLongFieldUpdater")
  object AtomicLongFieldUpdater:
    def newUpdater[U](tclass: Class[U], fieldName: String): AtomicLongFieldUpdater[U] =
      if fieldName == null then throw new NullPointerException()
      new LongFieldUpdater[U](fieldName)

  // The updater of the field `name`: its reads and writes are the interpreter's own (`fieldGet`,
  // `fieldCompareAndSet`, `fieldGetAndAdd`), which run whole.
  private[atomic] final class LongFieldUpdater[T](name: String) extends AtomicLongFieldUpdater[T]:
    def get(obj: T): Long = fieldGet(obj.asInstanceOf[AnyRef], name).asInstanceOf[Long]
    def set(obj: T, newValue: Long): Unit = fieldSet(obj.asInstanceOf[AnyRef], name, newValue)
    def compareAndSet(obj: T, expect: Long, update: Long): scala.Boolean =
      fieldCompareAndSet(obj.asInstanceOf[AnyRef], name, expect, update)
    def getAndSet(obj: T, newValue: Long): Long =
      var prev = get(obj)
      while !compareAndSet(obj, prev, newValue) do prev = get(obj)
      prev
    def getAndAdd(obj: T, delta: Long): Long =
      var prev = get(obj)
      while !compareAndSet(obj, prev, prev + delta) do prev = get(obj)
      prev

  @js("$fail(\"UnsupportedOperationException\", \"a field updater is not available on JavaScript\")")
  private[atomic] def fieldGet(obj: AnyRef, name: String): Any
  @js("$fail(\"UnsupportedOperationException\", \"a field updater is not available on JavaScript\")")
  private[atomic] def fieldSet(obj: AnyRef, name: String, v: Any): Unit
  @js("$fail(\"UnsupportedOperationException\", \"a field updater is not available on JavaScript\")")
  private[atomic] def fieldCompareAndSet(obj: AnyRef, name: String, expect: Any, update: Any): scala.Boolean

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
