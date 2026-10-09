// The queues, threads and executors of the JDK that library bodies name on JavaScript, which has
// one thread: Scala.js's javalib shapes, over the JS arrays of `java.util`'s store. The current
// thread is the one thread, named "main"; the runtime has one processor.
package java.lang:

  // A thread can be made but not started: `start` is the JDK's, which JavaScript has no
  // implementation of (a Scala.js program that starts one does not link).
  @jvmClass("java/lang/Thread")
  class Thread private (private var threadName: String, target: Runnable) extends Runnable:
    def this() = this("Thread-0", null)
    def this(target: Runnable) = this("Thread-0", target)
    def this(name: String) = this(name, null)
    def this(target: Runnable, name: String) = this(name, target)
    private var interruptedState = false
    def run(): Unit = if target != null then target.run()
    def interrupt(): Unit = interruptedState = true
    def isInterrupted(): scala.Boolean = interruptedState
    final def getName(): String = threadName
    final def setName(name: String): Unit = threadName = name
    def getId(): scala.Long = 1L
    final def isDaemon(): scala.Boolean = false
    final def isAlive(): scala.Boolean = true
    def getStackTrace(): Array[StackTraceElement] = new Array[StackTraceElement](0)

  @jvmClass("java/lang/Thread")
  object Thread:
    private val single = new Thread("main", null)
    def currentThread(): Thread = single
    def interrupted(): scala.Boolean =
      val was = single.interruptedState
      single.interruptedState = false
      was
    def onSpinWait(): Unit = ()
    trait UncaughtExceptionHandler:
      def uncaughtException(t: Thread, e: Throwable): Unit

  @jvmClass("java/lang/Runtime")
  class Runtime private ():
    def availableProcessors(): Int = 1
    def maxMemory(): scala.Long = scala.Long.MaxValue
    def freeMemory(): scala.Long = scala.Long.MaxValue
    def totalMemory(): scala.Long = scala.Long.MaxValue
    def gc(): Unit = ()

  @jvmClass("java/lang/Runtime")
  object Runtime:
    private val current = new Runtime()
    def getRuntime(): Runtime = current

package java.util:

  @jvmClass("java/util/SequencedCollection")
  trait SequencedCollection[E] extends Collection[E]:
    def reversed(): SequencedCollection[E]

  @jvmClass("java/util/Queue")
  trait Queue[E] extends Collection[E]:
    def offer(e: E): Boolean
    def poll(): E
    def peek(): E
    def element(): E
    def remove(): E

  @jvmClass("java/util/Deque")
  trait Deque[E] extends Queue[E], SequencedCollection[E]:
    def addFirst(e: E): Unit
    def addLast(e: E): Unit
    def offerFirst(e: E): Boolean
    def offerLast(e: E): Boolean
    def removeFirst(): E
    def removeLast(): E
    def pollFirst(): E
    def pollLast(): E
    def getFirst(): E
    def getLast(): E
    def peekFirst(): E
    def peekLast(): E
    def removeFirstOccurrence(o: Any): Boolean
    def removeLastOccurrence(o: Any): Boolean
    def push(e: E): Unit
    def pop(): E
    def descendingIterator(): Iterator[E]
    def reversed(): Deque[E]

  @jvmClass("java/util/AbstractCollection")
  abstract class AbstractCollection[E] extends Collection[E]

  @jvmClass("java/util/AbstractQueue")
  abstract class AbstractQueue[E] extends AbstractCollection[E], Queue[E]:
    override def add(e: E): Boolean =
      if offer(e) then true else throw new IllegalStateException("Queue full")
    def remove(): E =
      val e = poll()
      if e == null then throw new NoSuchElementException() else e
    def element(): E =
      val e = peek()
      if e == null then throw new NoSuchElementException() else e
    override def clear(): Unit = while poll() != null do ()
    override def addAll(c: Collection[? <: E]): Boolean =
      if c.asInstanceOf[AnyRef] eq this then throw new IllegalArgumentException()
      super.addAll(c)

  // The elements in order from the first; a `null` element is refused as the JDK's.
  @jvmClass("java/util/ArrayDeque")
  class ArrayDeque[E](private val items: Array[E]) extends AbstractCollection[E], Deque[E]:
    def this() = this(newArray[E])
    def this(numElements: Int) = this(newArray[E])
    def this(c: Collection[? <: E]) =
      this(newArray[E])
      addAll(c.asInstanceOf[Collection[E]])
    private def checked(e: E): E =
      if e == null then throw new NullPointerException() else e
    def size(): Int = items.length
    def iterator(): Iterator[E] = new DequeIterator(items, false)
    def descendingIterator(): Iterator[E] = new DequeIterator(items, true)
    def addFirst(e: E): Unit = arrayInsert(items, 0, checked(e))
    def addLast(e: E): Unit = arrayPush(items, checked(e))
    def offerFirst(e: E): Boolean =
      addFirst(e)
      true
    def offerLast(e: E): Boolean =
      addLast(e)
      true
    override def add(e: E): Boolean =
      addLast(e)
      true
    def offer(e: E): Boolean = offerLast(e)
    def push(e: E): Unit = addFirst(e)
    def pollFirst(): E =
      if items.length == 0 then null.asInstanceOf[E]
      else
        val e = items(0)
        arrayRemove(items, 0)
        e
    def pollLast(): E =
      if items.length == 0 then null.asInstanceOf[E]
      else
        val e = items(items.length - 1)
        arrayRemove(items, items.length - 1)
        e
    def removeFirst(): E =
      if items.length == 0 then throw new NoSuchElementException() else pollFirst()
    def removeLast(): E =
      if items.length == 0 then throw new NoSuchElementException() else pollLast()
    def poll(): E = pollFirst()
    def pop(): E = removeFirst()
    def remove(): E = removeFirst()
    def peekFirst(): E = if items.length == 0 then null.asInstanceOf[E] else items(0)
    def peekLast(): E = if items.length == 0 then null.asInstanceOf[E] else items(items.length - 1)
    def peek(): E = peekFirst()
    def getFirst(): E = if items.length == 0 then throw new NoSuchElementException() else items(0)
    def getLast(): E = if items.length == 0 then throw new NoSuchElementException() else items(items.length - 1)
    def element(): E = getFirst()
    def removeFirstOccurrence(o: Any): Boolean =
      var i = 0
      while i < items.length && !Objects.equals(items(i), o) do i += 1
      if i < items.length then
        arrayRemove(items, i)
        true
      else false
    def removeLastOccurrence(o: Any): Boolean =
      var i = items.length - 1
      while i >= 0 && !Objects.equals(items(i), o) do i -= 1
      if i >= 0 then
        arrayRemove(items, i)
        true
      else false
    override def remove(o: Any): Boolean = removeFirstOccurrence(o)
    override def clear(): Unit = arrayClear(items)
    override def retainAll(c: Collection[?]): Boolean =
      var changed = false
      var i = items.length - 1
      while i >= 0 do
        if !c.contains(items(i)) then
          arrayRemove(items, i)
          changed = true
        i -= 1
      changed
    def reversed(): Deque[E] =
      val out = new ArrayDeque[E]()
      var i = items.length - 1
      while i >= 0 do
        out.addLast(items(i))
        i -= 1
      out
    override def clone(): ArrayDeque[E] = new ArrayDeque[E](this)

  private final class DequeIterator[E](items: Array[E], descending: Boolean) extends Iterator[E]:
    private val snapshot = copyArray[E, E](items)
    private var i = 0
    private var last = -1
    def hasNext: Boolean = i < snapshot.length
    def next(): E =
      if i >= snapshot.length then throw new NoSuchElementException()
      last = if descending then snapshot.length - 1 - i else i
      i += 1
      snapshot(last)
    override def remove(): Unit =
      if last < 0 then throw new IllegalStateException()
      var j = 0
      while j < items.length && (items(j).asInstanceOf[AnyRef] ne snapshot(last).asInstanceOf[AnyRef]) do j += 1
      if j < items.length then arrayRemove(items, j)
      last = -1

package java.util.concurrent:

  @jvmClass("java/util/concurrent/Executor")
  trait Executor:
    def execute(command: Runnable): Unit

  class RejectedExecutionException(message: String = null, cause: Throwable = null) extends RuntimeException(message, cause)

  class CancellationException(message: String = null) extends IllegalStateException(message)

  class ExecutionException(message: String = null, cause: Throwable = null) extends Exception(message, cause)

  class TimeoutException(message: String = null) extends Exception(message)

  // A queue over a JS array, in order from the head; `null` is refused as the JDK's.
  @jvmClass("java/util/concurrent/ConcurrentLinkedQueue")
  class ConcurrentLinkedQueue[E](private val items: Array[E]) extends java.util.AbstractQueue[E]:
    def this() = this(java.util.newArray[E])
    def this(c: java.util.Collection[? <: E]) =
      this(java.util.newArray[E])
      addAll(c.asInstanceOf[java.util.Collection[E]])
    def size(): Int = items.length
    override def isEmpty(): Boolean = items.length == 0
    def offer(e: E): Boolean =
      if e == null then throw new NullPointerException()
      java.util.arrayPush(items, e)
      true
    def poll(): E =
      if items.length == 0 then null.asInstanceOf[E]
      else
        val e = items(0)
        java.util.arrayRemove(items, 0)
        e
    def peek(): E = if items.length == 0 then null.asInstanceOf[E] else items(0)
    override def remove(o: Any): Boolean =
      var i = 0
      while i < items.length && !java.util.Objects.equals(items(i), o) do i += 1
      if i < items.length then
        java.util.arrayRemove(items, i)
        true
      else false
    override def clear(): Unit = java.util.arrayClear(items)
    override def retainAll(c: java.util.Collection[?]): Boolean =
      var changed = false
      var i = items.length - 1
      while i >= 0 do
        if !c.contains(items(i)) then
          java.util.arrayRemove(items, i)
          changed = true
        i -= 1
      changed
    def iterator(): java.util.Iterator[E] = new java.util.DequeIterator(items, false)

  // java.util.Random's generator, as Scala.js's `ThreadLocalRandom` extends it.
  @jvmClass("java/util/concurrent/ThreadLocalRandom")
  final class ThreadLocalRandom private (private var seed: Long):
    private def next(bits: Int): Int =
      seed = (seed * 0x5DEECE66DL + 0xBL) & 0xFFFFFFFFFFFFL
      (seed >>> (48 - bits)).toInt
    def setSeed(value: Long): Unit = throw new UnsupportedOperationException()
    def nextInt(): Int = next(32)
    def nextInt(bound: Int): Int =
      if bound <= 0 then throw new IllegalArgumentException("bound must be positive")
      else if (bound & -bound) == bound then ((bound.toLong * next(31).toLong) >> 31).toInt
      else
        var bits = next(31)
        var value = bits % bound
        while bits - value + (bound - 1) < 0 do
          bits = next(31)
          value = bits % bound
        value
    def nextInt(origin: Int, bound: Int): Int =
      if origin >= bound then throw new IllegalArgumentException("bound must be greater than origin")
      val n = bound - origin
      if n > 0 then nextInt(n) + origin
      else
        var r = next(32)
        while r < origin || r >= bound do r = next(32)
        r
    def nextLong(): Long = (next(32).toLong << 32) + next(32).toLong
    def nextLong(bound: Long): Long =
      if bound <= 0 then throw new IllegalArgumentException("bound must be positive")
      val m = bound - 1
      var r = nextLong()
      if (bound & m) == 0L then r & m
      else
        var u = r >>> 1
        r = u % bound
        while u + m - r < 0L do
          u = nextLong() >>> 1
          r = u % bound
        r
    def nextDouble(): Double = ((next(26).toLong << 27) + next(27).toLong).toDouble / 9007199254740992.0
    def nextBoolean(): Boolean = next(1) != 0

  @jvmClass("java/util/concurrent/ThreadLocalRandom")
  object ThreadLocalRandom:
    private val single = new ThreadLocalRandom((freshSeed() ^ 0x5DEECE66DL) & 0xFFFFFFFFFFFFL)
    def current(): ThreadLocalRandom = single

  @js("BigInt(Math.floor(Math.random() * 281474976710656))")
  private def freshSeed(): Long
