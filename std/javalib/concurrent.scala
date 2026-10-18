// The queues, threads and executors of the JDK that library bodies name. On JavaScript, which has one
// thread, Scala.js's javalib shapes over the JS arrays of `java.util`'s store: the current thread is the
// one thread, named "main", which a thread made can never join, and the runtime has one processor. Under
// `teq interp` the threads are the interpreter's logical threads (src/interp/threads.rs), started, joined,
// slept, interrupted and parked as the JDK's: the defs with a `@js` template below are the layer's
// natives there, and on JavaScript the template's one thread.
package java.lang:

  // A thread: started under `teq interp`, where `start` makes a logical thread that runs `run`; on
  // JavaScript it can be made but not started (`start` is the JDK's, which JavaScript has no
  // implementation of: a Scala.js program that starts one does not link). `tid` is the interpreter's
  // number of the thread once started, -1 before (main's is 0).
  @jvmClass("java/lang/Thread")
  class Thread private (private var threadName: String, target: Runnable, group: ThreadGroup, private var tid: Int) extends Runnable:
    def this() = this(Thread.autoName(), null, null, -1)
    def this(target: Runnable) = this(Thread.autoName(), target, null, -1)
    def this(name: String) = this(name, null, null, -1)
    def this(target: Runnable, name: String) = this(name, target, null, -1)
    def this(group: ThreadGroup, target: Runnable) = this(Thread.autoName(), target, group, -1)
    def this(group: ThreadGroup, name: String) = this(name, null, group, -1)
    def this(group: ThreadGroup, target: Runnable, name: String) = this(name, target, group, -1)
    def this(group: ThreadGroup, target: Runnable, name: String, stackSize: scala.Long) = this(name, target, group, -1)
    // What a thread takes from the one that makes it, as the JDK's: its group, its daemon status, its
    // priority (at most the group's).
    private val parent: Thread = Thread.maker()
    private val threadGroup: ThreadGroup =
      if group != null then group else parent.getThreadGroup()
    private var daemon: scala.Boolean = parent != null && parent.isDaemon()
    private var priority: Int =
      if parent == null then Thread.NORM_PRIORITY else Math.min(parent.getPriority(), threadGroup.getMaxPriority())
    private var handler: Thread.UncaughtExceptionHandler = null
    // The interrupt where no logical thread holds it: JavaScript's, a compile-time run's, a thread's not
    // started yet.
    private var interruptFlag = false
    private val ident: scala.Long = Thread.nextIdent()
    // The maker's `InheritableThreadLocal` values, each through `childValue`, taken as the JDK's
    // constructor takes them (`ThreadLocal.createInheritedMap`) and the thread's at its start. Main takes
    // none: the JDK attaches it, a constructor that inherits nothing.
    private val inheritedLocals: AnyRef = if Thread.makingMain then null else threadInherited()
    if threadName == null then throw new NullPointerException("'name' is null")

    def run(): Unit = if target != null then target.run()

    def start(): Unit =
      if tid >= 0 then throw new IllegalThreadStateException()
      tid = threadStart(this, daemon, interruptFlag, inheritedLocals)

    def interrupt(): Unit =
      if tid < 0 || !threadsRun() then interruptFlag = true else threadInterrupt(tid)
    def isInterrupted(): scala.Boolean =
      if tid < 0 || !threadsRun() then interruptFlag else threadInterrupted(tid, false)
    // The interpreter's number of the thread, -1 before it starts (`LockSupport.unpark`).
    private[java] def nativeId: Int = tid
    final def getName(): String = threadName
    final def setName(name: String): Unit =
      if name == null then throw new NullPointerException("'name' is null")
      threadName = name
    @deprecated def getId(): scala.Long = ident
    final def threadId(): scala.Long = ident
    final def isDaemon(): scala.Boolean = daemon
    final def setDaemon(on: scala.Boolean): Unit =
      if isAlive() then throw new IllegalThreadStateException()
      daemon = on
    final def getPriority(): Int = priority
    final def setPriority(newPriority: Int): Unit =
      if newPriority > Thread.MAX_PRIORITY || newPriority < Thread.MIN_PRIORITY then
        throw new IllegalArgumentException()
      if isAlive() || tid < 0 then priority = Math.min(newPriority, threadGroup.getMaxPriority())
    // The group, `null` once the thread ended.
    final def getThreadGroup(): ThreadGroup = if getState() eq Thread.State.TERMINATED then null else threadGroup
    final def isAlive(): scala.Boolean = tid >= 0 && (!threadsRun() || threadState(tid) != 5)
    def getState(): Thread.State =
      if tid < 0 then Thread.State.NEW
      else if !threadsRun() then Thread.State.RUNNABLE
      else Thread.State.values()(threadState(tid))
    final def join(): Unit = join(0L)
    final def join(millis: scala.Long): Unit =
      if millis < 0L then throw new IllegalArgumentException("timeout value is negative")
      joinNanos(millis * 1000000L)
    final def join(millis: scala.Long, nanos: Int): Unit =
      if millis < 0L then throw new IllegalArgumentException("timeout value is negative")
      if nanos < 0 || nanos > 999999 then throw new IllegalArgumentException("nanosecond timeout value out of range")
      joinNanos(millis * 1000000L + nanos)
    private def joinNanos(nanos: scala.Long): Unit =
      if tid >= 0 && threadJoin(tid, nanos) == 2 then throw new InterruptedException()
    def getStackTrace(): Array[StackTraceElement] = new Array[StackTraceElement](0)
    def getUncaughtExceptionHandler(): Thread.UncaughtExceptionHandler =
      if handler != null then handler else if getState() eq Thread.State.TERMINATED then null else threadGroup
    def setUncaughtExceptionHandler(h: Thread.UncaughtExceptionHandler): Unit = handler = h
    def getContextClassLoader(): ClassLoader = null
    def setContextClassLoader(cl: ClassLoader): Unit = ()
    def checkAccess(): Unit = ()
    final def isVirtual(): scala.Boolean = false
    // What the thread's end does with an exception its `run` did not catch (the JDK's
    // `dispatchUncaughtException`, which the interpreter calls).
    private[lang] def dispatchUncaughtException(e: Throwable): Unit =
      getUncaughtExceptionHandler().uncaughtException(this, e)
    override def toString: String =
      val g = getThreadGroup()
      "Thread[#" + ident + "," + threadName + "," + priority + "," + (if g != null then g.getName() else "") + "]"

  // The logical threads' natives under `teq interp` (src/interp/threads.rs); JavaScript's one thread.
  @js("$fail(\"UnsupportedOperationException\", \"Thread.start is not available on JavaScript\")")
  private[lang] def threadStart(t: Thread, daemon: scala.Boolean, interrupted: scala.Boolean, locals: AnyRef): Int
  @js("null")
  private[lang] def threadInherited(): AnyRef
  @js("null")
  private[lang] def threadCurrent(): Thread
  @js("false")
  private[java] def threadsRun(): scala.Boolean
  @js("false")
  private[lang] def threadInterrupted(tid: Int, clear: scala.Boolean): scala.Boolean
  @js("void 0")
  private[lang] def threadInterrupt(tid: Int): Unit
  @js("1")
  private[lang] def threadState(tid: Int): Int
  @js("$fail(\"UnsupportedOperationException\", \"Thread.join is not available on JavaScript\")")
  private[lang] def threadJoin(tid: Int, nanos: scala.Long): Int
  @js("void 0")
  private[lang] def threadYield(): Unit
  @js("false")
  private[lang] def holdsLock(o: Object): scala.Boolean
  @js("$fail(\"UnsupportedOperationException\", \"Thread.sleep is not available on JavaScript\")")
  private[lang] def threadSleep(millis: scala.Long, nanos: scala.Long): Unit
  @js("false")
  private[lang] def monitorWait(o: Object, millis: scala.Long, nanos: scala.Long): scala.Boolean
  @js("void 0")
  private[lang] def monitorNotify(o: Object, all: scala.Boolean): Unit

  // `Object.wait`'s three overloads, `notify` and `notifyAll` of any reference, which a call typed to the
  // member is lowered to (`object_monitor_call`): under `teq interp` the waits and signals of the object's monitor,
  // which the thread has to hold (`IllegalMonitorStateException`); on JavaScript, which has one thread,
  // nothing, as Scala.js's; on the JVM the JDK's methods. A null receiver throws first, as the JVM's
  // `invokevirtual` does once the arguments are evaluated, then JDK 24's checks of the times.
  @js("void 0")
  @jvm("$0:L invokevirtual java/lang/Object.wait()V")
  def objectWait0(o: Object): Unit =
    monitorReceiver(o, "Object.wait()")
    monitorWaitMillis(o, 0L)
  @js("void 0")
  @jvm("$0:L $1:J invokevirtual java/lang/Object.wait(J)V")
  def objectWait1(o: Object, timeoutMillis: scala.Long): Unit =
    monitorReceiver(o, "Object.wait(long)")
    monitorWaitMillis(o, timeoutMillis)
  @js("void 0")
  @jvm("$0:L $1:J $2:I invokevirtual java/lang/Object.wait(JI)V")
  def objectWait(o: Object, timeoutMillis: scala.Long, nanos: Int): Unit =
    monitorReceiver(o, "Object.wait(long, int)")
    if timeoutMillis < 0L then throw new IllegalArgumentException("timeoutMillis value is negative")
    if nanos < 0 || nanos > 999999 then throw new IllegalArgumentException("nanosecond timeout value out of range")
    monitorWaitMillis(o, if nanos > 0 && timeoutMillis < java.lang.Long.MAX_VALUE then timeoutMillis + 1L else timeoutMillis)
  // `wait(long)`'s body.
  private def monitorWaitMillis(o: Object, timeoutMillis: scala.Long): Unit =
    if timeoutMillis < 0L then throw new IllegalArgumentException("timeout value is negative")
    if monitorWait(o, timeoutMillis, 0L) then throw new InterruptedException()
  private def monitorReceiver(o: Object, method: String): Unit =
    if o == null then throw new NullPointerException("Cannot invoke \"" + method + "\" because the value is null")
  @js("void 0")
  @jvm("$0:L invokevirtual java/lang/Object.notify()V")
  def objectNotify(o: Object): Unit =
    monitorReceiver(o, "Object.notify()")
    monitorNotify(o, false)
  @js("void 0")
  @jvm("$0:L invokevirtual java/lang/Object.notifyAll()V")
  def objectNotifyAll(o: Object): Unit =
    monitorReceiver(o, "Object.notifyAll()")
    monitorNotify(o, true)
  @js("$fail(\"UnsupportedOperationException\", \"shutdown hooks are not available on JavaScript\")")
  private[lang] def hookAdd(hook: Thread): Unit
  @js("$fail(\"UnsupportedOperationException\", \"shutdown hooks are not available on JavaScript\")")
  private[lang] def hookRemove(hook: Thread): scala.Boolean

  @jvmClass("java/lang/Thread")
  object Thread:
    final val MIN_PRIORITY = 1
    final val NORM_PRIORITY = 5
    final val MAX_PRIORITY = 10
    private var autoNumber = 0
    private var idents = 0L
    private var defaultHandler: UncaughtExceptionHandler = null
    private var systemGroup: ThreadGroup = null
    private var main: Thread = null
    private var makingMain = false
    private[lang] def autoName(): String =
      val n = autoNumber
      autoNumber += 1
      "Thread-" + n
    private[lang] def nextIdent(): scala.Long =
      idents += 1L
      idents
    // The thread making another: the current one, none while main itself is made.
    private[lang] def maker(): Thread = if makingMain then null else currentThread()
    def currentThread(): Thread =
      val t = threadCurrent()
      if t != null then t
      else
        if main == null then
          // Main's group, "main" in "system", made with it.
          makingMain = true
          systemGroup = new ThreadGroup(null, "system", true)
          main = new Thread("main", null, new ThreadGroup(systemGroup, "main"), 0)
          makingMain = false
        main
    // The current thread waits: under `teq interp` for the time, an interrupt ending it with
    // `InterruptedException` (a signal stops it), with the output so far flushed; JavaScript cannot
    // wait.
    def sleep(millis: scala.Long): Unit =
      if millis < 0L then throw new IllegalArgumentException("timeout value is negative")
      threadSleep(millis, 0L)
    def sleep(millis: scala.Long, nanos: Int): Unit =
      if millis < 0L then throw new IllegalArgumentException("timeout value is negative")
      if nanos < 0 || nanos > 999999 then throw new IllegalArgumentException("nanosecond timeout value out of range")
      threadSleep(millis, nanos.toLong)
    def interrupted(): scala.Boolean =
      val t = currentThread()
      if threadsRun() then threadInterrupted(-2, true)
      else
        val was = t.interruptFlag
        t.interruptFlag = false
        was
    def `yield`(): Unit = threadYield()
    def onSpinWait(): Unit = threadYield()
    def holdsLock(obj: Object): scala.Boolean =
      if obj == null then throw new NullPointerException()
      java.lang.holdsLock(obj)
    def getDefaultUncaughtExceptionHandler(): UncaughtExceptionHandler = defaultHandler
    def setDefaultUncaughtExceptionHandler(h: UncaughtExceptionHandler): Unit = defaultHandler = h
    def activeCount(): Int = 1
    def dumpStack(): Unit = new Exception("Stack trace").printStackTrace()

    trait UncaughtExceptionHandler:
      def uncaughtException(t: Thread, e: Throwable): Unit

    @jvmClass("java/lang/Thread$State")
    final class State private[lang] (label: String, position: Int) extends Enum[State]:
      override def name(): String = label
      override def ordinal(): Int = position
      override def toString: String = label
      override def compareTo(that: State): Int = position - that.ordinal()
      override def hashCode: Int = label.hashCode
      override def equals(that: Any): scala.Boolean = that.asInstanceOf[AnyRef] eq this

    @jvmClass("java/lang/Thread$State")
    object State:
      val NEW: State = new State("NEW", 0)
      val RUNNABLE: State = new State("RUNNABLE", 1)
      val BLOCKED: State = new State("BLOCKED", 2)
      val WAITING: State = new State("WAITING", 3)
      val TIMED_WAITING: State = new State("TIMED_WAITING", 4)
      val TERMINATED: State = new State("TERMINATED", 5)
      def values(): Array[State] = Array(NEW, RUNNABLE, BLOCKED, WAITING, TIMED_WAITING, TERMINATED)
      def valueOf(name: String): State =
        if name == null then throw new NullPointerException("Name is null")
        values().find(_.name() == name).getOrElse(throw new IllegalArgumentException("No enum constant java.lang.Thread.State." + name))

  // A group of threads, as the JDK's: a name, a parent, a maximum priority, and the handler of the
  // exceptions its threads do not catch, which reports them as the JDK's does when no default handler is
  // set.
  @jvmClass("java/lang/ThreadGroup")
  class ThreadGroup private[lang] (parent: ThreadGroup, name: String, root: scala.Boolean) extends Thread.UncaughtExceptionHandler:
    def this(parent: ThreadGroup, name: String) =
      this(if parent != null then parent else throw new NullPointerException(), name, false)
    def this(name: String) = this(Thread.currentThread().getThreadGroup(), name)
    private var maxPriority = if parent == null then Thread.MAX_PRIORITY else parent.getMaxPriority()
    private var daemon = parent != null && parent.isDaemon()
    final def getName(): String = name
    final def getParent(): ThreadGroup = parent
    final def getMaxPriority(): Int = maxPriority
    final def setMaxPriority(pri: Int): Unit =
      if pri >= Thread.MIN_PRIORITY && pri <= Thread.MAX_PRIORITY then
        maxPriority = if parent != null then Math.min(pri, parent.getMaxPriority()) else pri
    @deprecated final def isDaemon(): scala.Boolean = daemon
    @deprecated final def setDaemon(on: scala.Boolean): Unit = daemon = on
    final def parentOf(g: ThreadGroup): scala.Boolean =
      var c = g
      while c != null && (c ne this) do c = c.getParent()
      c != null
    def uncaughtException(t: Thread, e: Throwable): Unit =
      if parent != null then parent.uncaughtException(t, e)
      else
        val h = Thread.getDefaultUncaughtExceptionHandler()
        if h != null then h.uncaughtException(t, e)
        else
          System.err.print("Exception in thread \"" + t.getName() + "\" ")
          e.printStackTrace(System.err)
    override def toString: String = "java.lang.ThreadGroup[name=" + name + ",maxpri=" + maxPriority + "]"

  @jvmClass("java/lang/Runtime")
  class Runtime private ():
    // One on JavaScript and in a macro; the machine's under `teq interp`.
    @js("1")
    def availableProcessors(): Int
    // A thread whose `run` the end of a program under `teq interp` calls (`Interp::shut_down`):
    // after main returns, an uncaught exception, `System.exit` or a signal, as the JVM's shutdown
    // starts the hooks, here one after the other in the order they were added.
    def addShutdownHook(hook: Thread): Unit =
      if hook == null then throw new NullPointerException()
      hookAdd(hook)
    def removeShutdownHook(hook: Thread): scala.Boolean =
      if hook == null then throw new NullPointerException()
      hookRemove(hook)
    def exit(status: Int): Unit = System.exit(status)
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

  // The JDK's `AbstractCollection.remove` and `clear`: through the iterator.
  @jvmClass("java/util/AbstractCollection")
  abstract class AbstractCollection[E] extends Collection[E]:
    override def remove(o: Any): Boolean =
      val it = iterator()
      var found = false
      while !found && it.hasNext do
        if Objects.equals(o, it.next()) then
          it.remove()
          found = true
      found
    override def clear(): Unit =
      val it = iterator()
      while it.hasNext do
        it.next()
        it.remove()

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
    // The JDK's `bulkRemove`: every element tested first, in order, those selected removed after.
    override def removeIf(test: java.util.function.Predicate[? >: E]): Boolean =
      Objects.requireNonNull(test)
      bulkRemove(items, e => test.test(e))
    override def removeAll(c: Collection[?]): Boolean =
      Objects.requireNonNull(c)
      bulkRemove(items, e => c.contains(e))
    override def retainAll(c: Collection[?]): Boolean =
      Objects.requireNonNull(c)
      bulkRemove(items, e => !c.contains(e))
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

  // The reactive-streams interfaces, as Scala.js's javalib has them (fs2's `interop.flow`).
  @jvmClass("java/util/concurrent/Flow")
  object Flow:
    @jvmClass("java/util/concurrent/Flow$Publisher")
    trait Publisher[T]:
      def subscribe(subscriber: Subscriber[? >: T]): Unit
    @jvmClass("java/util/concurrent/Flow$Subscriber")
    trait Subscriber[T]:
      def onSubscribe(subscription: Subscription): Unit
      def onNext(item: T): Unit
      def onError(throwable: Throwable): Unit
      def onComplete(): Unit
    @jvmClass("java/util/concurrent/Flow$Subscription")
    trait Subscription:
      def request(n: scala.Long): Unit
      def cancel(): Unit
    @jvmClass("java/util/concurrent/Flow$Processor")
    trait Processor[T, R] extends Subscriber[T], Publisher[R]
    def defaultBufferSize(): Int = 256

  class RejectedExecutionException(message: String = null, cause: Throwable = null) extends RuntimeException(message, cause)

  class CancellationException(message: String = null) extends IllegalStateException(message)

  class ExecutionException(message: String = null, cause: Throwable = null) extends Exception(message, cause):
    def this(cause: Throwable) = this(if cause == null then null else cause.toString, cause)

  class TimeoutException(message: String = null) extends Exception(message)

  // A queue over a JS array, in order from the head; `null` is refused as the JDK's.
  @jvmClass("java/util/concurrent/ConcurrentLinkedQueue")
  class ConcurrentLinkedQueue[E](private val items: Array[E]) extends java.util.AbstractQueue[E]:
    def this() = this(java.util.newArray[E])
    def this(c: java.util.Collection[? <: E]) =
      this(java.util.newArray[E])
      addAll(c.asInstanceOf[java.util.Collection[E]])
    // Each operation under the queue's monitor, which under `teq interp`'s threads makes it whole;
    // JavaScript's one thread takes none.
    def size(): Int = synchronized(items.length)
    override def isEmpty(): Boolean = synchronized(items.length == 0)
    def offer(e: E): Boolean =
      if e == null then throw new NullPointerException()
      synchronized(java.util.arrayPush(items, e))
      true
    def poll(): E = synchronized:
      if items.length == 0 then null.asInstanceOf[E]
      else
        val e = items(0)
        java.util.arrayRemove(items, 0)
        e
    def peek(): E = synchronized(if items.length == 0 then null.asInstanceOf[E] else items(0))
    override def remove(o: Any): Boolean = synchronized:
      var i = 0
      while i < items.length && !java.util.Objects.equals(items(i), o) do i += 1
      if i < items.length then
        java.util.arrayRemove(items, i)
        true
      else false
    override def clear(): Unit = synchronized(java.util.arrayClear(items))
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
    private val single = new ThreadLocalRandom((java.util.freshSeed() ^ 0x5DEECE66DL) & 0xFFFFFFFFFFFFL)
    def current(): ThreadLocalRandom = single
