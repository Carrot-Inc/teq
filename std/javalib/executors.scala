// The JDK's blocking queues, latches, semaphores, futures and executors, written from its documented
// behaviour over the locks and conditions of `java.util.concurrent.locks` (std/javalib/locks.scala), as
// the JDK's own sit over its `AbstractQueuedSynchronizer`: under `teq interp` their threads are the
// interpreter's logical threads; on JavaScript, which has one thread, a wait that nothing could end fails.
// On the JVM the classes are the JDK's own (`@jvmClass`).
package java.util.concurrent:

  import java.util.concurrent.locks.{Condition, ReentrantLock}

  @jvmClass("java/util/concurrent/Callable")
  trait Callable[V]:
    def call(): V

  @jvmClass("java/util/concurrent/Future")
  trait Future[V]:
    def cancel(mayInterruptIfRunning: scala.Boolean): scala.Boolean
    def isCancelled(): scala.Boolean
    def isDone(): scala.Boolean
    def get(): V
    def get(timeout: scala.Long, unit: TimeUnit): V

  @jvmClass("java/util/concurrent/RunnableFuture")
  trait RunnableFuture[V] extends Runnable, Future[V]

  @jvmClass("java/util/concurrent/Delayed")
  trait Delayed extends java.lang.Comparable[Delayed]:
    def getDelay(unit: TimeUnit): scala.Long

  @jvmClass("java/util/concurrent/ScheduledFuture")
  trait ScheduledFuture[V] extends Delayed, Future[V]

  @jvmClass("java/util/concurrent/RunnableScheduledFuture")
  trait RunnableScheduledFuture[V] extends RunnableFuture[V], ScheduledFuture[V]:
    def isPeriodic(): scala.Boolean

  @jvmClass("java/util/concurrent/ThreadFactory")
  trait ThreadFactory:
    def newThread(r: Runnable): Thread

  @jvmClass("java/util/concurrent/RejectedExecutionHandler")
  trait RejectedExecutionHandler:
    def rejectedExecution(r: Runnable, executor: ThreadPoolExecutor): Unit

  // A computation run once, by `run`, whose result or exception `get` waits for; a cancel before it
  // ends ends it with `CancellationException`, interrupting the thread running it when asked.
  @jvmClass("java/util/concurrent/FutureTask")
  class FutureTask[V](private var callable: Callable[V]) extends RunnableFuture[V]:
    if callable == null then throw new NullPointerException()
    def this(runnable: Runnable, result: V) = this(Executors.callable(runnable, result))
    // NEW, COMPLETING, NORMAL, EXCEPTIONAL, CANCELLED, INTERRUPTING, INTERRUPTED: the JDK's states.
    private var state = 0
    private var outcome: Any = null
    private var runner: Thread = null
    private val lock = new ReentrantLock()
    private val ended = lock.newCondition()

    def isCancelled(): scala.Boolean = state >= 4
    def isDone(): scala.Boolean = state != 0
    def cancel(mayInterruptIfRunning: scala.Boolean): scala.Boolean =
      lock.lock()
      val cancelled =
        try
          if state != 0 then false
          else
            state = if mayInterruptIfRunning then 5 else 4
            if mayInterruptIfRunning then
              if runner != null then runner.interrupt()
              state = 6
            ended.signalAll()
            true
        finally lock.unlock()
      if cancelled then done()
      cancelled
    private def report(s: Int): V =
      if s == 2 then outcome.asInstanceOf[V]
      else if s >= 4 then throw new CancellationException()
      else throw new ExecutionException(outcome.asInstanceOf[Throwable])
    def get(): V =
      lock.lock()
      try
        while state <= 1 do ended.await()
        report(state)
      finally lock.unlock()
    def get(timeout: scala.Long, unit: TimeUnit): V =
      if unit == null then throw new NullPointerException()
      lock.lock()
      try
        var nanos = unit.toNanos(timeout)
        while state <= 1 && nanos > 0L do nanos = ended.awaitNanos(nanos)
        if state <= 1 then throw new TimeoutException()
        report(state)
      finally lock.unlock()
    def resultNow(): V =
      if state == 2 then outcome.asInstanceOf[V] else throw new IllegalStateException()
    protected def done(): Unit = ()
    protected def set(v: V): Unit =
      if finish(2, v) then done()
    protected def setException(t: Throwable): Unit =
      if finish(3, t) then done()
    private def finish(s: Int, value: Any): scala.Boolean =
      lock.lock()
      try
        if state != 0 then false
        else
          outcome = value
          state = s
          ended.signalAll()
          true
      finally lock.unlock()
    def run(): Unit =
      if state != 0 || runner != null then return
      runner = Thread.currentThread()
      try
        val c = callable
        if c != null && state == 0 then
          var ok = false
          var result: V = null.asInstanceOf[V]
          try
            result = c.call()
            ok = true
          catch case e: Throwable => setException(e)
          if ok then set(result)
      finally
        runner = null
        possibleCancellationInterrupt()
    // JDK 24's `handlePossibleCancellationInterrupt`: a `cancel(true)` still interrupting the runner is
    // waited out, and its interrupt is left to the runner, which cannot be told from another.
    private def possibleCancellationInterrupt(): Unit =
      while state == 5 do Thread.`yield`()
    // A periodic task's run, which leaves the task able to run again unless it failed or was
    // cancelled.
    protected def runAndReset(): scala.Boolean =
      if state != 0 || runner != null then return false
      runner = Thread.currentThread()
      var ran = false
      try
        val c = callable
        if c != null && state == 0 then
          try
            c.call()
            ran = true
          catch case e: Throwable => setException(e)
      finally
        runner = null
        possibleCancellationInterrupt()
      ran && state == 0
    override def toString: String =
      val status = state match
        case 2 => "[Completed normally]"
        case 3 => "[Completed exceptionally: " + outcome + "]"
        case s if s >= 4 => "[Cancelled]"
        case _ => if callable == null then "[Not completed]" else "[Not completed, task = " + callable + "]"
      super.toString + status

  @jvmClass("java/util/concurrent/BlockingQueue")
  trait BlockingQueue[E] extends java.util.Queue[E]:
    def add(e: E): scala.Boolean
    def offer(e: E): scala.Boolean
    def remove(o: Any): scala.Boolean
    def contains(o: Any): scala.Boolean
    def put(e: E): Unit
    def offer(e: E, timeout: scala.Long, unit: TimeUnit): scala.Boolean
    def take(): E
    def poll(timeout: scala.Long, unit: TimeUnit): E
    def remainingCapacity(): Int
    def drainTo(c: java.util.Collection[? >: E]): Int
    def drainTo(c: java.util.Collection[? >: E], maxElements: Int): Int

  // A queue of at most `capacity` elements, in order, over one lock: `put` waits for room, `take` for
  // an element, the timed `offer` and `poll` for their time at most.
  @jvmClass("java/util/concurrent/LinkedBlockingQueue")
  class LinkedBlockingQueue[E](capacity: Int) extends java.util.AbstractQueue[E], BlockingQueue[E], java.io.Serializable:
    if capacity <= 0 then throw new IllegalArgumentException()
    def this() = this(Integer.MAX_VALUE)
    def this(c: java.util.Collection[? <: E]) =
      this(Integer.MAX_VALUE)
      val it = c.iterator()
      while it.hasNext do put(it.next())
    private val items = new java.util.ArrayDeque[E]()
    private val lock = new ReentrantLock()
    private val notEmpty = lock.newCondition()
    private val notFull = lock.newCondition()

    def size(): Int =
      lock.lock()
      try items.size() finally lock.unlock()
    def remainingCapacity(): Int =
      lock.lock()
      try capacity - items.size() finally lock.unlock()
    private def enqueue(e: E): Unit =
      items.addLast(e)
      notEmpty.signal()
    private def dequeue(): E =
      val e = items.pollFirst()
      notFull.signal()
      e
    def put(e: E): Unit =
      if e == null then throw new NullPointerException()
      lock.lockInterruptibly()
      try
        while items.size() == capacity do notFull.await()
        enqueue(e)
      finally lock.unlock()
    def offer(e: E, timeout: scala.Long, unit: TimeUnit): scala.Boolean =
      if e == null then throw new NullPointerException()
      var nanos = unit.toNanos(timeout)
      lock.lockInterruptibly()
      try
        while items.size() == capacity && nanos > 0L do nanos = notFull.awaitNanos(nanos)
        if items.size() == capacity then false
        else
          enqueue(e)
          true
      finally lock.unlock()
    def offer(e: E): scala.Boolean =
      if e == null then throw new NullPointerException()
      lock.lock()
      try
        if items.size() == capacity then false
        else
          enqueue(e)
          true
      finally lock.unlock()
    def take(): E =
      lock.lockInterruptibly()
      try
        while items.isEmpty() do notEmpty.await()
        dequeue()
      finally lock.unlock()
    def poll(timeout: scala.Long, unit: TimeUnit): E =
      var nanos = unit.toNanos(timeout)
      lock.lockInterruptibly()
      try
        while items.isEmpty() && nanos > 0L do nanos = notEmpty.awaitNanos(nanos)
        if items.isEmpty() then null.asInstanceOf[E] else dequeue()
      finally lock.unlock()
    def poll(): E =
      lock.lock()
      try if items.isEmpty() then null.asInstanceOf[E] else dequeue()
      finally lock.unlock()
    def peek(): E =
      lock.lock()
      try items.peekFirst() finally lock.unlock()
    override def remove(o: Any): scala.Boolean =
      lock.lock()
      try
        val removed = o != null && items.removeFirstOccurrence(o)
        if removed then notFull.signal()
        removed
      finally lock.unlock()
    override def contains(o: Any): scala.Boolean =
      lock.lock()
      try o != null && items.contains(o) finally lock.unlock()
    override def clear(): Unit =
      lock.lock()
      try
        items.clear()
        notFull.signalAll()
      finally lock.unlock()
    def drainTo(c: java.util.Collection[? >: E]): Int = drainTo(c, Integer.MAX_VALUE)
    def drainTo(c: java.util.Collection[? >: E], maxElements: Int): Int =
      if c == null then throw new NullPointerException()
      if c.asInstanceOf[AnyRef] eq this then throw new IllegalArgumentException()
      lock.lock()
      try
        var n = 0
        while n < maxElements && !items.isEmpty() do
          c.asInstanceOf[java.util.Collection[E]].add(items.pollFirst())
          n += 1
        if n > 0 then notFull.signalAll()
        n
      finally lock.unlock()
    // The elements when it is called, in order.
    def iterator(): java.util.Iterator[E] =
      lock.lock()
      val snapshot =
        try new java.util.ArrayList[E](items)
        finally lock.unlock()
      val each = snapshot.iterator()
      new java.util.Iterator[E]:
        private var last: E = null.asInstanceOf[E]
        def hasNext: scala.Boolean = each.hasNext
        def next(): E =
          last = each.next()
          last
        override def remove(): Unit =
          if last == null then throw new IllegalStateException()
          LinkedBlockingQueue.this.remove(last)
          last = null.asInstanceOf[E]
    override def toString: String =
      lock.lock()
      try items.toString finally lock.unlock()

  // A queue that holds nothing: an element offered passes to a thread taking at the same time, and an
  // `offer` without a taker waiting fails.
  @jvmClass("java/util/concurrent/SynchronousQueue")
  class SynchronousQueue[E](fair: scala.Boolean) extends java.util.AbstractQueue[E], BlockingQueue[E], java.io.Serializable:
    def this() = this(false)
    private val lock = new ReentrantLock()
    private val changed = lock.newCondition()
    // JDK 24's `Transferer`, under the lock: the waiting nodes, all of one mode (a node of the other
    // would have matched), the newest first unless fair. A node is an element given (`isData`) or a
    // request; the thread that matches it takes it off and sets its item, the waiter's own the cancelling.
    private final class Node(val isData: scala.Boolean, var item: AnyRef):
      var matched = false
    private val waiting = new java.util.ArrayDeque[Node]()

    // `Transferer.xferLifo` (`xfer` when fair): `e` an element given, or null to take one, matched to the
    // first waiting node of the other mode, else waiting `nanos` (0 not at all, `Long.MAX_VALUE` for
    // ever) as `DualNode.await` does, an interrupt or the time cancelling. The match, or `e` if none.
    private def xfer(e: AnyRef, nanos: scala.Long): AnyRef =
      val haveData = e != null
      lock.lock()
      try
        val p = waiting.peekFirst()
        if p != null && p.isData != haveData then
          waiting.pollFirst()
          val m = p.item
          p.item = e
          p.matched = true
          changed.signalAll()
          m
        else if nanos == 0L then e
        else
          val s = new Node(haveData, e)
          if fair then waiting.addLast(s) else waiting.addFirst(s)
          val timed = nanos != java.lang.Long.MAX_VALUE
          var ns = nanos
          var cancelled = false
          while !s.matched && !cancelled do
            if Thread.currentThread().isInterrupted() || (timed && ns <= 0L) then
              waiting.remove(s)
              cancelled = true
            else
              try if timed then ns = changed.awaitNanos(ns) else changed.await()
              catch case _: InterruptedException => Thread.currentThread().interrupt()
          if cancelled then e else s.item
      finally lock.unlock()

    def size(): Int = 0
    def remainingCapacity(): Int = 0
    def iterator(): java.util.Iterator[E] = java.util.Collections.emptyIterator[E]()
    def peek(): E = null.asInstanceOf[E]
    override def isEmpty(): scala.Boolean = true
    def offer(e: E): scala.Boolean =
      if e == null then throw new NullPointerException()
      xfer(e.asInstanceOf[AnyRef], 0L) == null
    def offer(e: E, timeout: scala.Long, unit: TimeUnit): scala.Boolean =
      if e == null then throw new NullPointerException()
      if xfer(e.asInstanceOf[AnyRef], Math.max(unit.toNanos(timeout), 0L)) == null then true
      else if !Thread.interrupted() then false
      else throw new InterruptedException()
    def put(e: E): Unit =
      if e == null then throw new NullPointerException()
      if Thread.interrupted() || xfer(e.asInstanceOf[AnyRef], java.lang.Long.MAX_VALUE) != null then
        Thread.interrupted()
        throw new InterruptedException()
    def take(): E =
      val e = if Thread.interrupted() then null else xfer(null, java.lang.Long.MAX_VALUE)
      if e == null then
        Thread.interrupted()
        throw new InterruptedException()
      e.asInstanceOf[E]
    def poll(timeout: scala.Long, unit: TimeUnit): E =
      val e = xfer(null, Math.max(unit.toNanos(timeout), 0L))
      if e != null || !Thread.interrupted() then e.asInstanceOf[E] else throw new InterruptedException()
    def poll(): E = xfer(null, 0L).asInstanceOf[E]
    def drainTo(c: java.util.Collection[? >: E]): Int = 0
    def drainTo(c: java.util.Collection[? >: E], maxElements: Int): Int = 0
    override def toString: String = "[]"

  // A count that threads wait to see at zero.
  @jvmClass("java/util/concurrent/CountDownLatch")
  class CountDownLatch(count: Int):
    if count < 0 then throw new IllegalArgumentException("count < 0")
    private var remaining = count
    private val lock = new ReentrantLock()
    private val zero = lock.newCondition()

    // JDK 24's `acquireSharedInterruptibly` and `tryAcquireSharedNanos`: the interrupt checked at the start
    // and again as each wait returns, before the count, so that an interrupt that came while the thread
    // waited ends its wait though the count reached zero meanwhile.
    def await(): Unit =
      if Thread.interrupted() then throw new InterruptedException()
      lock.lock()
      try
        while remaining > 0 do
          zero.await()
          if Thread.interrupted() then throw new InterruptedException()
      finally lock.unlock()
    def await(timeout: scala.Long, unit: TimeUnit): scala.Boolean =
      if Thread.interrupted() then throw new InterruptedException()
      lock.lock()
      try
        var nanos = unit.toNanos(timeout)
        while remaining > 0 && nanos > 0L do
          nanos = zero.awaitNanos(nanos)
          if Thread.interrupted() then throw new InterruptedException()
        remaining == 0
      finally lock.unlock()
    def countDown(): Unit =
      lock.lock()
      try
        if remaining > 0 then
          remaining -= 1
          if remaining == 0 then zero.signalAll()
      finally lock.unlock()
    def getCount(): scala.Long =
      lock.lock()
      try remaining.toLong finally lock.unlock()
    override def toString: String = super.toString + "[Count = " + getCount() + "]"

  // Permits that threads take and give back, a taker waiting while too few are left.
  @jvmClass("java/util/concurrent/Semaphore")
  class Semaphore(permits: Int, fair: scala.Boolean) extends java.io.Serializable:
    def this(permits: Int) = this(permits, false)
    private var available = permits
    private val lock = new ReentrantLock()
    private val released = lock.newCondition()

    private def take(n: Int, interruptible: scala.Boolean, nanos0: scala.Long): scala.Boolean =
      if n < 0 then throw new IllegalArgumentException()
      if interruptible && Thread.interrupted() then throw new InterruptedException()
      lock.lock()
      try
        var nanos = nanos0
        while available < n && (nanos0 < 0L || nanos > 0L) do
          if nanos0 < 0L then (if interruptible then released.await() else released.awaitUninterruptibly())
          else nanos = released.awaitNanos(nanos)
          // The interrupt checked as each wait returns, before the permits, as `CountDownLatch.await`'s.
          if interruptible && Thread.interrupted() then throw new InterruptedException()
        if available >= n then
          available -= n
          true
        else false
      finally lock.unlock()
    def acquire(): Unit = take(1, true, -1L)
    def acquire(n: Int): Unit = take(n, true, -1L)
    def acquireUninterruptibly(): Unit = take(1, false, -1L)
    def acquireUninterruptibly(n: Int): Unit = take(n, false, -1L)
    def tryAcquire(): scala.Boolean = take(1, false, 0L)
    def tryAcquire(n: Int): scala.Boolean = take(n, false, 0L)
    def tryAcquire(timeout: scala.Long, unit: TimeUnit): scala.Boolean = take(1, true, Math.max(unit.toNanos(timeout), 0L))
    def tryAcquire(n: Int, timeout: scala.Long, unit: TimeUnit): scala.Boolean = take(n, true, Math.max(unit.toNanos(timeout), 0L))
    def release(): Unit = release(1)
    def release(n: Int): Unit =
      if n < 0 then throw new IllegalArgumentException()
      lock.lock()
      try
        available += n
        released.signalAll()
      finally lock.unlock()
    def availablePermits(): Int =
      lock.lock()
      try available finally lock.unlock()
    def drainPermits(): Int =
      lock.lock()
      try
        val n = Math.max(available, 0)
        available -= n
        n
      finally lock.unlock()
    def isFair(): scala.Boolean = fair
    override def toString: String = super.toString + "[Permits = " + availablePermits() + "]"

  @jvmClass("java/util/concurrent/ExecutorService")
  trait ExecutorService extends Executor, AutoCloseable:
    def shutdown(): Unit
    def shutdownNow(): java.util.List[Runnable]
    def isShutdown(): scala.Boolean
    def isTerminated(): scala.Boolean
    def awaitTermination(timeout: scala.Long, unit: TimeUnit): scala.Boolean
    def submit[T](task: Callable[T]): Future[T]
    def submit[T](task: Runnable, result: T): Future[T]
    def submit(task: Runnable): Future[?]
    def invokeAll[T](tasks: java.util.Collection[? <: Callable[T]]): java.util.List[Future[T]]
    def invokeAny[T](tasks: java.util.Collection[? <: Callable[T]]): T
    def close(): Unit =
      if !isTerminated() then
        shutdown()
        while !awaitTermination(1L, TimeUnit.DAYS) do ()

  @jvmClass("java/util/concurrent/ScheduledExecutorService")
  trait ScheduledExecutorService extends ExecutorService:
    def schedule(command: Runnable, delay: scala.Long, unit: TimeUnit): ScheduledFuture[?]
    def schedule[V](callable: Callable[V], delay: scala.Long, unit: TimeUnit): ScheduledFuture[V]
    def scheduleAtFixedRate(command: Runnable, initialDelay: scala.Long, period: scala.Long, unit: TimeUnit): ScheduledFuture[?]
    def scheduleWithFixedDelay(command: Runnable, initialDelay: scala.Long, delay: scala.Long, unit: TimeUnit): ScheduledFuture[?]

  @jvmClass("java/util/concurrent/AbstractExecutorService")
  abstract class AbstractExecutorService extends ExecutorService:
    protected def newTaskFor[T](runnable: Runnable, value: T): RunnableFuture[T] = new FutureTask[T](runnable, value)
    protected def newTaskFor[T](callable: Callable[T]): RunnableFuture[T] = new FutureTask[T](callable)
    def submit(task: Runnable): Future[?] =
      if task == null then throw new NullPointerException()
      val f = newTaskFor[Any](task, null)
      execute(f)
      f
    def submit[T](task: Runnable, result: T): Future[T] =
      if task == null then throw new NullPointerException()
      val f = newTaskFor[T](task, result)
      execute(f)
      f
    def submit[T](task: Callable[T]): Future[T] =
      if task == null then throw new NullPointerException()
      val f = newTaskFor[T](task)
      execute(f)
      f
    def invokeAll[T](tasks: java.util.Collection[? <: Callable[T]]): java.util.List[Future[T]] =
      if tasks == null then throw new NullPointerException()
      val futures = new java.util.ArrayList[Future[T]]()
      val it = tasks.iterator()
      while it.hasNext do
        val f = newTaskFor[T](it.next())
        futures.add(f)
        execute(f)
      val each = futures.iterator()
      while each.hasNext do
        val f = each.next()
        if !f.isDone() then
          try f.get()
          catch
            case _: CancellationException => ()
            case _: ExecutionException => ()
      futures
    def invokeAny[T](tasks: java.util.Collection[? <: Callable[T]]): T =
      val futures = invokeAll[T](tasks)
      var last: ExecutionException = null
      val it = futures.iterator()
      while it.hasNext do
        try return it.next().get()
        catch case e: ExecutionException => last = e
      throw (if last != null then last else new ExecutionException(null))

  // A pool of threads that run the tasks given it, as the JDK's: up to `corePoolSize` threads made for
  // the tasks as they come, then the tasks queued, then threads up to `maximumPoolSize` while the queue
  // refuses them, then the handler's refusal; a thread past the core size (or any, when the core times
  // out) ends after `keepAliveTime` without a task. `shutdown` takes no new task and lets the queued
  // ones run; `shutdownNow` interrupts the threads and hands the queued ones back.
  @jvmClass("java/util/concurrent/ThreadPoolExecutor")
  class ThreadPoolExecutor(
      private var corePoolSize: Int,
      private var maximumPoolSize: Int,
      keepAliveTime: scala.Long,
      keepAliveUnit: TimeUnit,
      workQueue: BlockingQueue[Runnable],
      private var threadFactory: ThreadFactory,
      private var handler: RejectedExecutionHandler
  ) extends AbstractExecutorService:
    if corePoolSize < 0 || maximumPoolSize <= 0 || maximumPoolSize < corePoolSize || keepAliveTime < 0L then
      throw new IllegalArgumentException()
    if workQueue == null || threadFactory == null || handler == null then throw new NullPointerException()
    def this(core: Int, max: Int, keepAlive: scala.Long, unit: TimeUnit, queue: BlockingQueue[Runnable]) =
      this(core, max, keepAlive, unit, queue, Executors.defaultThreadFactory(), ThreadPoolExecutor.defaultHandler)
    def this(core: Int, max: Int, keepAlive: scala.Long, unit: TimeUnit, queue: BlockingQueue[Runnable], factory: ThreadFactory) =
      this(core, max, keepAlive, unit, queue, factory, ThreadPoolExecutor.defaultHandler)
    def this(core: Int, max: Int, keepAlive: scala.Long, unit: TimeUnit, queue: BlockingQueue[Runnable], handler: RejectedExecutionHandler) =
      this(core, max, keepAlive, unit, queue, Executors.defaultThreadFactory(), handler)

    private var keepAliveNanos = keepAliveUnit.toNanos(keepAliveTime)
    private var coreTimesOut = false
    // RUNNING 0, SHUTDOWN 1, STOP 2, TERMINATED 3.
    private var runState = 0
    private val mainLock = new ReentrantLock()
    private val termination = mainLock.newCondition()
    private val workers = new java.util.HashSet[ThreadPoolExecutor.Worker]()
    private var workerCount = 0
    private var active = 0
    private var largestPoolSize = 0
    private var completedTaskCount = 0L

    def execute(command: Runnable): Unit =
      if command == null then throw new NullPointerException()
      if runState == 0 && workerCountNow() < corePoolSize && addWorker(command, true) then return
      if runState == 0 && workQueue.offer(command) then
        if runState != 0 && workQueue.remove(command) then reject(command)
        else if workerCountNow() == 0 then addWorker(null, false)
      else if !addWorker(command, false) then reject(command)

    private def workerCountNow(): Int =
      mainLock.lock()
      try workerCount finally mainLock.unlock()

    private[concurrent] def reject(command: Runnable): Unit = handler.rejectedExecution(command, this)

    private[concurrent] def isStopped(): scala.Boolean = runState >= 2
    // A thread for the queued tasks when there are fewer than the core's, or none (`ensurePrestart`).
    private[concurrent] def ensurePrestart(): Unit =
      val n = workerCountNow()
      if n < corePoolSize then addWorker(null, true)
      else if n == 0 then addWorker(null, false)

    private def addWorker(first: Runnable, core: scala.Boolean): scala.Boolean =
      mainLock.lock()
      val w =
        try
          if runState >= 2 || (runState == 1 && (first != null || workQueue.isEmpty())) then null
          else if workerCount >= (if core then corePoolSize else maximumPoolSize) then null
          else
            val w = new ThreadPoolExecutor.Worker(this, first)
            val t = threadFactory.newThread(w)
            if t == null then null
            else
              w.thread = t
              workers.add(w)
              workerCount += 1
              if workerCount > largestPoolSize then largestPoolSize = workerCount
              w
        finally mainLock.unlock()
      if w == null then false
      else
        w.thread.start()
        true

    // A thread's next task, or null when the thread is to end: the pool stopping, shut down with an
    // empty queue, or idle past its keep-alive while it is surplus.
    private[concurrent] def getTask(): Runnable =
      var timedOut = false
      while true do
        mainLock.lock()
        val (stop, timed) =
          try
            val stopNow = runState >= 2 || (runState == 1 && workQueue.isEmpty())
            val timed = coreTimesOut || workerCount > corePoolSize
            if stopNow || ((workerCount > maximumPoolSize || (timed && timedOut)) && (workerCount > 1 || workQueue.isEmpty())) then
              workerCount -= 1
              (true, timed)
            else (false, timed)
          finally mainLock.unlock()
        if stop then return null
        try
          val r = if timed then workQueue.poll(keepAliveNanos, TimeUnit.NANOSECONDS) else workQueue.take()
          if r != null then return r
          timedOut = true
        catch case _: InterruptedException => timedOut = false
      null

    private[concurrent] def runWorker(w: ThreadPoolExecutor.Worker): Unit =
      var task = w.firstTask
      w.firstTask = null
      var abrupt = true
      try
        while task != null || { task = getTask(); task != null } do
          if runState >= 2 && !w.thread.isInterrupted() then w.thread.interrupt()
          else if runState < 2 then Thread.interrupted()
          mainLock.lock()
          try
            active += 1
            w.busy = true
          finally mainLock.unlock()
          try
            beforeExecute(w.thread, task)
            try
              task.run()
              afterExecute(task, null)
            catch
              case e: Throwable =>
                afterExecute(task, e)
                throw e
          finally
            mainLock.lock()
            try
              active -= 1
              w.busy = false
              completedTaskCount += 1
            finally mainLock.unlock()
            task = null
        abrupt = false
      finally workerExit(w, abrupt)

    private def workerExit(w: ThreadPoolExecutor.Worker, abrupt: scala.Boolean): Unit =
      mainLock.lock()
      try
        if abrupt then workerCount -= 1
        workers.remove(w)
      finally mainLock.unlock()
      tryTerminate()
      if runState < 2 then
        val min = if coreTimesOut then (if workQueue.isEmpty() then 0 else 1) else corePoolSize
        if abrupt || workerCountNow() < min then addWorker(null, false)

    private[concurrent] def tryTerminate(): Unit =
      mainLock.lock()
      try
        if runState == 1 && workQueue.isEmpty() || runState == 2 then
          if workerCount == 0 then
            runState = 3
            try terminated()
            finally termination.signalAll()
          else interruptIdle(true)
      finally mainLock.unlock()

    // The threads not running a task interrupted, as the JDK's `interruptIdleWorkers` (whose worker
    // holds its own lock while it runs one).
    private def interruptIdle(onlyOne: scala.Boolean): Unit =
      val it = workers.iterator()
      var done = false
      while !done && it.hasNext do
        val w = it.next()
        val t = w.thread
        if t != null && !w.busy && !t.isInterrupted() then t.interrupt()
        if onlyOne then done = true

    protected def beforeExecute(t: Thread, r: Runnable): Unit = ()
    protected def afterExecute(r: Runnable, t: Throwable): Unit = ()
    protected def terminated(): Unit = ()

    def shutdown(): Unit =
      mainLock.lock()
      try
        if runState < 1 then runState = 1
        interruptIdle(false)
      finally mainLock.unlock()
      onShutdown()
      tryTerminate()
    private[concurrent] def onShutdown(): Unit = ()
    def shutdownNow(): java.util.List[Runnable] =
      val left = new java.util.ArrayList[Runnable]()
      mainLock.lock()
      try
        if runState < 2 then runState = 2
        val it = workers.iterator()
        while it.hasNext do
          val t = it.next().thread
          if t != null then t.interrupt()
        drainQueue(left)
      finally mainLock.unlock()
      tryTerminate()
      left
    // JDK 24's `drainQueue`: the queue drained, then what its `drainTo` left (a delay queue's tasks not yet
    // due) removed one by one.
    private def drainQueue(left: java.util.List[Runnable]): Unit =
      workQueue.drainTo(left)
      if !workQueue.isEmpty() then
        val rest = workQueue.toArray()
        var i = 0
        while i < rest.length do
          val r = rest(i).asInstanceOf[Runnable]
          if workQueue.remove(r) then left.add(r)
          i += 1
    def isShutdown(): scala.Boolean = runState >= 1
    def isTerminating(): scala.Boolean = runState >= 1 && runState < 3
    def isTerminated(): scala.Boolean = runState == 3
    def awaitTermination(timeout: scala.Long, unit: TimeUnit): scala.Boolean =
      var nanos = unit.toNanos(timeout)
      mainLock.lock()
      try
        while runState < 3 && nanos > 0L do nanos = termination.awaitNanos(nanos)
        runState == 3
      finally mainLock.unlock()

    def setThreadFactory(f: ThreadFactory): Unit =
      if f == null then throw new NullPointerException()
      threadFactory = f
    def getThreadFactory(): ThreadFactory = threadFactory
    def setRejectedExecutionHandler(h: RejectedExecutionHandler): Unit =
      if h == null then throw new NullPointerException()
      handler = h
    def getRejectedExecutionHandler(): RejectedExecutionHandler = handler
    def setCorePoolSize(n: Int): Unit =
      if n < 0 || maximumPoolSize < n then throw new IllegalArgumentException()
      corePoolSize = n
    def getCorePoolSize(): Int = corePoolSize
    def prestartCoreThread(): scala.Boolean = workerCountNow() < corePoolSize && addWorker(null, true)
    def prestartAllCoreThreads(): Int =
      var n = 0
      while addWorker(null, true) do n += 1
      n
    def allowsCoreThreadTimeOut(): scala.Boolean = coreTimesOut
    def allowCoreThreadTimeOut(value: scala.Boolean): Unit =
      if value && keepAliveNanos <= 0L then throw new IllegalArgumentException("Core threads must have nonzero keep alive times")
      coreTimesOut = value
    def setMaximumPoolSize(n: Int): Unit =
      if n <= 0 || n < corePoolSize then throw new IllegalArgumentException()
      maximumPoolSize = n
    def getMaximumPoolSize(): Int = maximumPoolSize
    def setKeepAliveTime(time: scala.Long, unit: TimeUnit): Unit =
      if time < 0L then throw new IllegalArgumentException()
      if time == 0L && coreTimesOut then throw new IllegalArgumentException("Core threads must have nonzero keep alive times")
      keepAliveNanos = unit.toNanos(time)
    def getKeepAliveTime(unit: TimeUnit): scala.Long = unit.convert(keepAliveNanos, TimeUnit.NANOSECONDS)
    def getQueue(): BlockingQueue[Runnable] = workQueue
    def remove(task: Runnable): scala.Boolean =
      val removed = workQueue.remove(task)
      tryTerminate()
      removed
    def purge(): Unit = ()
    def getPoolSize(): Int = if runState == 3 then 0 else workerCountNow()
    def getActiveCount(): Int =
      mainLock.lock()
      try active finally mainLock.unlock()
    def getLargestPoolSize(): Int = largestPoolSize
    def getTaskCount(): scala.Long = completedTaskCount + getActiveCount() + workQueue.size()
    def getCompletedTaskCount(): scala.Long = completedTaskCount
    override def toString: String =
      val state = if runState == 0 then "Running" else if runState == 3 then "Terminated" else "Shutting down"
      super.toString + "[" + state + ", pool size = " + getPoolSize() + ", active threads = " + getActiveCount() +
        ", queued tasks = " + workQueue.size() + ", completed tasks = " + completedTaskCount + "]"

  @jvmClass("java/util/concurrent/ThreadPoolExecutor")
  object ThreadPoolExecutor:
    private[concurrent] final class Worker(val pool: ThreadPoolExecutor, var firstTask: Runnable) extends Runnable:
      var thread: Thread = null
      // Whether it runs a task, when an interrupt of the idle ones leaves it alone.
      var busy = false
      def run(): Unit = pool.runWorker(this)
    private[concurrent] val defaultHandler: RejectedExecutionHandler = new AbortPolicy()

    @jvmClass("java/util/concurrent/ThreadPoolExecutor$AbortPolicy")
    class AbortPolicy extends RejectedExecutionHandler:
      def rejectedExecution(r: Runnable, e: ThreadPoolExecutor): Unit =
        throw new RejectedExecutionException("Task " + r.toString + " rejected from " + e.toString)
    @jvmClass("java/util/concurrent/ThreadPoolExecutor$CallerRunsPolicy")
    class CallerRunsPolicy extends RejectedExecutionHandler:
      def rejectedExecution(r: Runnable, e: ThreadPoolExecutor): Unit = if !e.isShutdown() then r.run()
    @jvmClass("java/util/concurrent/ThreadPoolExecutor$DiscardPolicy")
    class DiscardPolicy extends RejectedExecutionHandler:
      def rejectedExecution(r: Runnable, e: ThreadPoolExecutor): Unit = ()
    @jvmClass("java/util/concurrent/ThreadPoolExecutor$DiscardOldestPolicy")
    class DiscardOldestPolicy extends RejectedExecutionHandler:
      def rejectedExecution(r: Runnable, e: ThreadPoolExecutor): Unit =
        if !e.isShutdown() then
          e.getQueue().poll()
          e.execute(r)

  // A pool whose tasks run after their delay, and the periodic ones again and again, from a queue of
  // tasks by the time they are due; its threads are its core size, made as tasks come.
  @jvmClass("java/util/concurrent/ScheduledThreadPoolExecutor")
  class ScheduledThreadPoolExecutor(core: Int, factory: ThreadFactory, rejection: RejectedExecutionHandler)
      extends ThreadPoolExecutor(core, Integer.MAX_VALUE, 10L, TimeUnit.MILLISECONDS, new ScheduledThreadPoolExecutor.DelayedWorkQueue(), factory, rejection),
        ScheduledExecutorService:
    def this(core: Int) = this(core, Executors.defaultThreadFactory(), ThreadPoolExecutor.defaultHandler)
    def this(core: Int, factory: ThreadFactory) = this(core, factory, ThreadPoolExecutor.defaultHandler)
    def this(core: Int, rejection: RejectedExecutionHandler) = this(core, Executors.defaultThreadFactory(), rejection)

    private var continuePeriodic = false
    private var executeDelayed = true
    private var removeOnCancel = false
    private var sequence = 0L

    private def queue: ScheduledThreadPoolExecutor.DelayedWorkQueue = getQueue().asInstanceOf[ScheduledThreadPoolExecutor.DelayedWorkQueue]
    private def triggerTime(delay: scala.Long, unit: TimeUnit): scala.Long = System.nanoTime() + Math.max(unit.toNanos(delay), 0L)
    private def delayedExecute(task: ScheduledThreadPoolExecutor.ScheduledFutureTask[?]): Unit =
      if isShutdown() then reject(task)
      else
        queue.add(task)
        if !canRunInCurrentRunState(task) && remove(task) then task.cancel(false)
        else ensurePrestart()
    private[concurrent] def canRunInCurrentRunState(task: ScheduledThreadPoolExecutor.ScheduledFutureTask[?]): scala.Boolean =
      if !isShutdown() then true
      else if isStopped() then false
      else if task.isPeriodic() then continuePeriodic
      else executeDelayed || task.getDelay(TimeUnit.NANOSECONDS) <= 0L
    private[concurrent] def reExecutePeriodic(task: ScheduledThreadPoolExecutor.ScheduledFutureTask[?]): Unit =
      if canRunInCurrentRunState(task) then
        queue.add(task)
        if canRunInCurrentRunState(task) || !remove(task) then ensurePrestart()
        else task.cancel(false)
      else task.cancel(false)
    private[concurrent] def cancelled(task: ScheduledThreadPoolExecutor.ScheduledFutureTask[?]): Unit =
      if removeOnCancel then remove(task)
    private def nextSequence(): scala.Long =
      sequence += 1L
      sequence

    def schedule(command: Runnable, delay: scala.Long, unit: TimeUnit): ScheduledFuture[?] =
      if command == null || unit == null then throw new NullPointerException()
      val t = new ScheduledThreadPoolExecutor.ScheduledFutureTask[Any](this, Executors.callable(command, null), triggerTime(delay, unit), 0L, nextSequence())
      delayedExecute(t)
      t
    def schedule[V](callable: Callable[V], delay: scala.Long, unit: TimeUnit): ScheduledFuture[V] =
      if callable == null || unit == null then throw new NullPointerException()
      val t = new ScheduledThreadPoolExecutor.ScheduledFutureTask[V](this, callable, triggerTime(delay, unit), 0L, nextSequence())
      delayedExecute(t)
      t
    def scheduleAtFixedRate(command: Runnable, initialDelay: scala.Long, period: scala.Long, unit: TimeUnit): ScheduledFuture[?] =
      if command == null || unit == null then throw new NullPointerException()
      if period <= 0L then throw new IllegalArgumentException()
      val t = new ScheduledThreadPoolExecutor.ScheduledFutureTask[Any](this, Executors.callable(command, null), triggerTime(initialDelay, unit), unit.toNanos(period), nextSequence())
      delayedExecute(t)
      t
    def scheduleWithFixedDelay(command: Runnable, initialDelay: scala.Long, delay: scala.Long, unit: TimeUnit): ScheduledFuture[?] =
      if command == null || unit == null then throw new NullPointerException()
      if delay <= 0L then throw new IllegalArgumentException()
      val t = new ScheduledThreadPoolExecutor.ScheduledFutureTask[Any](this, Executors.callable(command, null), triggerTime(initialDelay, unit), -unit.toNanos(delay), nextSequence())
      delayedExecute(t)
      t
    override def execute(command: Runnable): Unit = schedule(command, 0L, TimeUnit.NANOSECONDS)
    override def submit(task: Runnable): Future[?] = schedule(task, 0L, TimeUnit.NANOSECONDS)
    override def submit[T](task: Runnable, result: T): Future[T] = schedule(Executors.callable(task, result), 0L, TimeUnit.NANOSECONDS)
    override def submit[T](task: Callable[T]): Future[T] = schedule(task, 0L, TimeUnit.NANOSECONDS)
    // JDK 24's setters: a policy disabled after the shutdown cancels the tasks it no longer keeps, as the
    // shutdown would have (`onShutdown`, with its termination check).
    def setContinueExistingPeriodicTasksAfterShutdownPolicy(value: scala.Boolean): Unit =
      continuePeriodic = value
      if !value && isShutdown() then onShutdown()
    def getContinueExistingPeriodicTasksAfterShutdownPolicy(): scala.Boolean = continuePeriodic
    def setExecuteExistingDelayedTasksAfterShutdownPolicy(value: scala.Boolean): Unit =
      executeDelayed = value
      if !value && isShutdown() then onShutdown()
    def getExecuteExistingDelayedTasksAfterShutdownPolicy(): scala.Boolean = executeDelayed
    def setRemoveOnCancelPolicy(value: scala.Boolean): Unit = removeOnCancel = value
    def getRemoveOnCancelPolicy(): scala.Boolean = removeOnCancel
    // The delayed tasks that may not run after the shutdown are cancelled then, as the JDK's
    // `onShutdown` cancels them, then checks the pool's termination.
    override private[concurrent] def onShutdown(): Unit =
      val it = queue.snapshot().iterator()
      while it.hasNext do
        val t = it.next().asInstanceOf[ScheduledThreadPoolExecutor.ScheduledFutureTask[?]]
        val drop = if t.isPeriodic() then !continuePeriodic else !executeDelayed && t.getDelay(TimeUnit.NANOSECONDS) > 0L
        if (drop || t.isCancelled()) && queue.remove(t) then t.cancel(false)
      tryTerminate()

  @jvmClass("java/util/concurrent/ScheduledThreadPoolExecutor")
  object ScheduledThreadPoolExecutor:
    // A delayed task: due at `time` (`System.nanoTime`), periodic at a fixed rate (`period` > 0) or a
    // fixed delay (< 0), ordered by time and then by its sequence.
    private[concurrent] class ScheduledFutureTask[V](pool: ScheduledThreadPoolExecutor, callable: Callable[V], private var time: scala.Long, period: scala.Long, val sequence: scala.Long)
        extends FutureTask[V](callable), RunnableScheduledFuture[V]:
      def getDelay(unit: TimeUnit): scala.Long = unit.convert(time - System.nanoTime(), TimeUnit.NANOSECONDS)
      def compareTo(other: Delayed): Int =
        if other.asInstanceOf[AnyRef] eq this then 0
        else other match
          case x: ScheduledFutureTask[?] =>
            val diff = time - x.dueAt
            if diff < 0L then -1 else if diff > 0L then 1 else if sequence < x.sequence then -1 else 1
          case _ =>
            val diff = getDelay(TimeUnit.NANOSECONDS) - other.getDelay(TimeUnit.NANOSECONDS)
            if diff < 0L then -1 else if diff > 0L then 1 else 0
      private[concurrent] def dueAt: scala.Long = time
      def isPeriodic(): scala.Boolean = period != 0L
      override def cancel(mayInterruptIfRunning: scala.Boolean): scala.Boolean =
        val cancelled = super.cancel(mayInterruptIfRunning)
        if cancelled then pool.cancelled(this)
        cancelled
      override def run(): Unit =
        if !pool.canRunInCurrentRunState(this) then cancel(false)
        else if !isPeriodic() then super.run()
        else if runAndReset() then
          time = if period > 0L then time + period else System.nanoTime() - period
          pool.reExecutePeriodic(this)

    // The pool's queue: its tasks by due time, `take` waiting for the first to be due.
    private[concurrent] class DelayedWorkQueue extends java.util.AbstractQueue[Runnable], BlockingQueue[Runnable]:
      private val tasks = new java.util.ArrayList[Runnable]()
      private val lock = new ReentrantLock()
      private val available = lock.newCondition()
      private def due(r: Runnable): scala.Long = r.asInstanceOf[Delayed].getDelay(TimeUnit.NANOSECONDS)
      private def insert(r: Runnable): Unit =
        var i = 0
        while i < tasks.size() && tasks.get(i).asInstanceOf[Delayed].compareTo(r.asInstanceOf[Delayed]) <= 0 do i += 1
        tasks.add(i, r)
        if i == 0 then available.signalAll()
      def size(): Int =
        lock.lock()
        try tasks.size() finally lock.unlock()
      def remainingCapacity(): Int = Integer.MAX_VALUE
      private[concurrent] def snapshot(): java.util.List[Runnable] =
        lock.lock()
        try new java.util.ArrayList[Runnable](tasks) finally lock.unlock()
      def iterator(): java.util.Iterator[Runnable] = snapshot().iterator()
      def offer(r: Runnable): scala.Boolean =
        if r == null then throw new NullPointerException()
        lock.lock()
        try
          insert(r)
          true
        finally lock.unlock()
      def put(r: Runnable): Unit = offer(r)
      override def add(r: Runnable): scala.Boolean = offer(r)
      def offer(r: Runnable, timeout: scala.Long, unit: TimeUnit): scala.Boolean = offer(r)
      def peek(): Runnable =
        lock.lock()
        try if tasks.isEmpty() then null else tasks.get(0) finally lock.unlock()
      def poll(): Runnable =
        lock.lock()
        try if tasks.isEmpty() || due(tasks.get(0)) > 0L then null else tasks.remove(0)
        finally lock.unlock()
      def take(): Runnable =
        lock.lockInterruptibly()
        try
          var r: Runnable = null
          while r == null do
            if tasks.isEmpty() then available.await()
            else
              val delay = due(tasks.get(0))
              if delay <= 0L then r = tasks.remove(0)
              else available.awaitNanos(delay)
          if !tasks.isEmpty() then available.signalAll()
          r
        finally lock.unlock()
      def poll(timeout: scala.Long, unit: TimeUnit): Runnable =
        var nanos = unit.toNanos(timeout)
        lock.lockInterruptibly()
        try
          var r: Runnable = null
          var giveUp = false
          while r == null && !giveUp do
            if tasks.isEmpty() then
              if nanos <= 0L then giveUp = true else nanos = available.awaitNanos(nanos)
            else
              val delay = due(tasks.get(0))
              if delay <= 0L then r = tasks.remove(0)
              else if nanos <= 0L then giveUp = true
              else
                val waited = Math.min(nanos, delay)
                val left = available.awaitNanos(waited)
                nanos -= waited - left
          if !tasks.isEmpty() then available.signalAll()
          r
        finally lock.unlock()
      override def remove(o: Any): scala.Boolean =
        lock.lock()
        try tasks.remove(o) finally lock.unlock()
      override def contains(o: Any): scala.Boolean =
        lock.lock()
        try tasks.contains(o) finally lock.unlock()
      override def clear(): Unit =
        lock.lock()
        try tasks.clear() finally lock.unlock()
      def drainTo(c: java.util.Collection[? >: Runnable]): Int = drainTo(c, Integer.MAX_VALUE)
      def drainTo(c: java.util.Collection[? >: Runnable], maxElements: Int): Int =
        lock.lock()
        try
          var n = 0
          while n < maxElements && !tasks.isEmpty() && due(tasks.get(0)) <= 0L do
            c.asInstanceOf[java.util.Collection[Runnable]].add(tasks.remove(0))
            n += 1
          n
        finally lock.unlock()

  @jvmClass("java/util/concurrent/Executors")
  object Executors:
    private var poolNumber = 0
    // The JDK's default factory: threads named `pool-<n>-thread-<m>`, not daemons, of normal priority,
    // in the group of the thread that made the factory.
    private final class DefaultThreadFactory extends ThreadFactory:
      poolNumber += 1
      private val prefix = "pool-" + poolNumber + "-thread-"
      private val group = Thread.currentThread().getThreadGroup()
      private var threadNumber = 0
      def newThread(r: Runnable): Thread =
        threadNumber += 1
        val t = new Thread(group, r, prefix + threadNumber, 0L)
        if t.isDaemon() then t.setDaemon(false)
        if t.getPriority() != Thread.NORM_PRIORITY then t.setPriority(Thread.NORM_PRIORITY)
        t
    def defaultThreadFactory(): ThreadFactory = new DefaultThreadFactory()
    def callable[T](task: Runnable, result: T): Callable[T] =
      if task == null then throw new NullPointerException()
      new Callable[T]:
        def call(): T =
          task.run()
          result
        override def toString: String = super.toString + "[Wrapped task = " + task + "]"
    def callable(task: Runnable): Callable[Object] = callable[Object](task, null)
    def newFixedThreadPool(n: Int): ExecutorService =
      new ThreadPoolExecutor(n, n, 0L, TimeUnit.MILLISECONDS, new LinkedBlockingQueue[Runnable]())
    def newFixedThreadPool(n: Int, factory: ThreadFactory): ExecutorService =
      new ThreadPoolExecutor(n, n, 0L, TimeUnit.MILLISECONDS, new LinkedBlockingQueue[Runnable](), factory)
    def newSingleThreadExecutor(): ExecutorService = newFixedThreadPool(1)
    def newSingleThreadExecutor(factory: ThreadFactory): ExecutorService = newFixedThreadPool(1, factory)
    def newCachedThreadPool(): ExecutorService =
      new ThreadPoolExecutor(0, Integer.MAX_VALUE, 60L, TimeUnit.SECONDS, new SynchronousQueue[Runnable]())
    def newCachedThreadPool(factory: ThreadFactory): ExecutorService =
      new ThreadPoolExecutor(0, Integer.MAX_VALUE, 60L, TimeUnit.SECONDS, new SynchronousQueue[Runnable](), factory)
    def newScheduledThreadPool(core: Int): ScheduledExecutorService = new ScheduledThreadPoolExecutor(core)
    def newScheduledThreadPool(core: Int, factory: ThreadFactory): ScheduledExecutorService = new ScheduledThreadPoolExecutor(core, factory)
    def newSingleThreadScheduledExecutor(): ScheduledExecutorService = new ScheduledThreadPoolExecutor(1)
    def newSingleThreadScheduledExecutor(factory: ThreadFactory): ScheduledExecutorService = new ScheduledThreadPoolExecutor(1, factory)

  @jvmClass("java/util/concurrent/CompletionException")
  class CompletionException(message: String, cause: Throwable) extends RuntimeException(message, cause):
    def this(cause: Throwable) = this(if cause == null then null else cause.toString, cause)
    def this(message: String) = this(message, null)

  @jvmClass("java/util/concurrent/CompletionStage")
  trait CompletionStage[T]:
    def toCompletableFuture(): CompletableFuture[T]

  // A future completed by whoever calls `complete`, and the stages that follow from it, run by the
  // thread that completes it (or that adds them to one already complete), as the JDK's run.
  @jvmClass("java/util/concurrent/CompletableFuture")
  class CompletableFuture[T] extends Future[T], CompletionStage[T]:
    // 0 pending, 1 a value, 2 an exception (the JDK's `AltResult`).
    private var state = 0
    private var value: Any = null
    private val lock = new ReentrantLock()
    private val ended = lock.newCondition()
    private val stages = new java.util.ArrayList[Runnable]()

    private def finish(s: Int, v: Any): scala.Boolean =
      lock.lock()
      val run =
        try
          if state != 0 then null
          else
            state = s
            value = v
            ended.signalAll()
            val r = new java.util.ArrayList[Runnable](stages)
            stages.clear()
            r
        finally lock.unlock()
      if run == null then false
      else
        val it = run.iterator()
        while it.hasNext do it.next().run()
        true
    // Runs `r` once this one is complete: now, or by its completion.
    private def onDone(r: Runnable): Unit =
      lock.lock()
      val now =
        try
          if state == 0 then
            stages.add(r)
            false
          else true
        finally lock.unlock()
      if now then r.run()
    private def failure: Throwable = value.asInstanceOf[Throwable]
    private def wrapped(t: Throwable): Throwable = if t.isInstanceOf[CompletionException] then t else new CompletionException(t)

    def complete(v: T): scala.Boolean = finish(1, v)
    def completeExceptionally(ex: Throwable): scala.Boolean =
      if ex == null then throw new NullPointerException()
      finish(2, ex)
    def cancel(mayInterruptIfRunning: scala.Boolean): scala.Boolean =
      val cancelled = state == 0 && finish(2, new CancellationException())
      cancelled || isCancelled()
    def isDone(): scala.Boolean = state != 0
    def isCancelled(): scala.Boolean = state == 2 && value.isInstanceOf[CancellationException]
    def isCompletedExceptionally(): scala.Boolean = state == 2
    private def waitDone(): Unit =
      lock.lock()
      try while state == 0 do ended.await()
      finally lock.unlock()
    def get(): T =
      waitDone()
      reportGet()
    def get(timeout: scala.Long, unit: TimeUnit): T =
      var nanos = unit.toNanos(timeout)
      lock.lock()
      try while state == 0 && nanos > 0L do nanos = ended.awaitNanos(nanos)
      finally lock.unlock()
      if state == 0 then throw new TimeoutException()
      reportGet()
    private def reportGet(): T =
      if state == 1 then value.asInstanceOf[T]
      else
        val t = failure
        t match
          case c: CancellationException => throw c
          case c: CompletionException if c.getCause != null => throw new ExecutionException(c.getCause)
          case _ => throw new ExecutionException(t)
    def join(): T =
      lock.lock()
      try while state == 0 do ended.awaitUninterruptibly()
      finally lock.unlock()
      if state == 1 then value.asInstanceOf[T]
      else
        failure match
          case c: CancellationException => throw c
          case t => throw wrapped(t)
    def getNow(valueIfAbsent: T): T = if state == 0 then valueIfAbsent else join()
    def toCompletableFuture(): CompletableFuture[T] = this

    def thenApply[U](fn: java.util.function.Function[? >: T, ? <: U]): CompletableFuture[U] =
      val next = new CompletableFuture[U]()
      onDone(() =>
        if state == 2 then next.completeExceptionally(wrapped(failure))
        else
          try next.complete(fn.asInstanceOf[java.util.function.Function[T, U]].apply(value.asInstanceOf[T]))
          catch case e: Throwable => next.completeExceptionally(wrapped(e))
      )
      next
    def thenAccept(action: java.util.function.Consumer[? >: T]): CompletableFuture[Void] =
      thenApply[Void](v => { action.asInstanceOf[java.util.function.Consumer[T]].accept(v); null })
    def thenRun(action: Runnable): CompletableFuture[Void] = thenApply[Void](_ => { action.run(); null })
    def thenCompose[U](fn: java.util.function.Function[? >: T, ? <: CompletionStage[U]]): CompletableFuture[U] =
      val next = new CompletableFuture[U]()
      onDone(() =>
        if state == 2 then next.completeExceptionally(wrapped(failure))
        else
          try
            val stage = fn.asInstanceOf[java.util.function.Function[T, CompletionStage[U]]].apply(value.asInstanceOf[T]).toCompletableFuture()
            stage.onDone(() =>
              if stage.state == 2 then next.completeExceptionally(wrapped(stage.failure))
              else next.complete(stage.value.asInstanceOf[U])
            )
          catch case e: Throwable => next.completeExceptionally(wrapped(e))
      )
      next
    def whenComplete(action: java.util.function.BiConsumer[? >: T, ? >: Throwable]): CompletableFuture[T] =
      val next = new CompletableFuture[T]()
      onDone(() =>
        val (v, ex) = if state == 1 then (value.asInstanceOf[T], null) else (null.asInstanceOf[T], failure)
        try
          action.asInstanceOf[java.util.function.BiConsumer[T, Throwable]].accept(v, ex)
          if ex == null then next.complete(v) else next.completeExceptionally(wrapped(ex))
        catch case e: Throwable => next.completeExceptionally(wrapped(if ex != null then ex else e))
      )
      next
    def handle[U](fn: java.util.function.BiFunction[? >: T, Throwable, ? <: U]): CompletableFuture[U] =
      val next = new CompletableFuture[U]()
      onDone(() =>
        val (v, ex) = if state == 1 then (value.asInstanceOf[T], null) else (null.asInstanceOf[T], failure)
        try next.complete(fn.asInstanceOf[java.util.function.BiFunction[T, Throwable, U]].apply(v, ex))
        catch case e: Throwable => next.completeExceptionally(wrapped(e))
      )
      next
    def exceptionally(fn: java.util.function.Function[Throwable, ? <: T]): CompletableFuture[T] =
      val next = new CompletableFuture[T]()
      onDone(() =>
        if state == 1 then next.complete(value.asInstanceOf[T])
        else
          try next.complete(fn.asInstanceOf[java.util.function.Function[Throwable, T]].apply(failure))
          catch case e: Throwable => next.completeExceptionally(wrapped(e))
      )
      next
    override def toString: String =
      val status = state match
        case 0 => if stages.isEmpty() then "[Incomplete]" else "[Incomplete with " + stages.size() + " dependents]"
        case 1 => "[Completed normally]"
        case _ => "[Completed exceptionally: " + value + "]"
      super.toString + status

  @jvmClass("java/util/concurrent/CompletableFuture")
  object CompletableFuture:
    // The JDK's default for the async methods, its common pool: daemon threads named
    // `ForkJoinPool.commonPool-worker-<n>`, made as tasks come and ended when idle.
    private var common: ExecutorService = null
    private var commonWorkers = 0
    private def asyncPool: ExecutorService =
      if common == null then
        val factory = new ThreadFactory:
          def newThread(r: Runnable): Thread =
            commonWorkers += 1
            val t = new Thread(r, "ForkJoinPool.commonPool-worker-" + commonWorkers)
            t.setDaemon(true)
            t
        common = new ThreadPoolExecutor(0, Integer.MAX_VALUE, 60L, TimeUnit.SECONDS, new SynchronousQueue[Runnable](), factory)
      common
    def supplyAsync[U](supplier: java.util.function.Supplier[U]): CompletableFuture[U] = supplyAsync(supplier, asyncPool)
    def supplyAsync[U](supplier: java.util.function.Supplier[U], executor: Executor): CompletableFuture[U] =
      if supplier == null || executor == null then throw new NullPointerException()
      val f = new CompletableFuture[U]()
      executor.execute(() =>
        try f.complete(supplier.get())
        catch case e: Throwable => f.completeExceptionally(new CompletionException(e))
      )
      f
    def runAsync(runnable: Runnable): CompletableFuture[Void] = runAsync(runnable, asyncPool)
    def runAsync(runnable: Runnable, executor: Executor): CompletableFuture[Void] =
      supplyAsync[Void](() => { runnable.run(); null }, executor)
    def completedFuture[U](value: U): CompletableFuture[U] =
      val f = new CompletableFuture[U]()
      f.complete(value)
      f
    def failedFuture[U](ex: Throwable): CompletableFuture[U] =
      val f = new CompletableFuture[U]()
      f.completeExceptionally(ex)
      f
