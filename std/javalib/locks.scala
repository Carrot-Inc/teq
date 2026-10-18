// `java.util.concurrent.locks`: under `teq interp` the locks, conditions and parks of the interpreter's
// logical threads (src/interp/threads.rs), which the defs with a `@js` template below are the natives of;
// on JavaScript, which has one thread, a lock counts the one thread's holds and a wait that nothing could
// end fails. On the JVM the classes are the JDK's own (`@jvmClass`).
package java.util.concurrent.locks:

  import java.util.concurrent.TimeUnit

  @js("$fail(\"UnsupportedOperationException\", \"LockSupport.park is not available on JavaScript\")")
  private[locks] def parkNative(nanos: scala.Long): Unit
  @js("void 0")
  private[locks] def unparkNative(tid: Int): Unit
  @js("0")
  private[locks] def lockAcquire(sync: Object, interruptible: scala.Boolean, nanos: scala.Long): Int
  @js("void 0")
  private[locks] def lockRelease(sync: Object): Unit
  @js("0")
  private[locks] def lockHolds(sync: Object): Int
  @js("0")
  private[locks] def lockState(sync: Object, queued: scala.Boolean): Int
  @js("$fail(\"UnsupportedOperationException\", \"Condition.await is not available on JavaScript\")")
  private[locks] def conditionAwait(sync: Object, cond: Int, nanos: scala.Long, interruptible: scala.Boolean): Int
  @js("void 0")
  private[locks] def conditionSignal(sync: Object, cond: Int, all: scala.Boolean): Unit

  // The JDK's parks: a thread waits until its permit is given (`unpark`), it is interrupted (which
  // stays set), its time is up, or for no reason.
  @jvmClass("java/util/concurrent/locks/LockSupport")
  object LockSupport:
    def park(): Unit = parkNative(-1L)
    def park(blocker: Object): Unit = parkNative(-1L)
    def parkNanos(nanos: scala.Long): Unit = if nanos > 0L then parkNative(nanos)
    def parkNanos(blocker: Object, nanos: scala.Long): Unit = if nanos > 0L then parkNative(nanos)
    // Until the absolute time in milliseconds since the epoch.
    def parkUntil(deadline: scala.Long): Unit =
      val millis = deadline - System.currentTimeMillis()
      if millis > 0L then parkNative(millis * 1000000L)
    def parkUntil(blocker: Object, deadline: scala.Long): Unit = parkUntil(deadline)
    def unpark(thread: Thread): Unit = if thread != null then unparkNative(thread.nativeId)
    def getBlocker(t: Thread): Object =
      if t == null then throw new NullPointerException()
      null

  @jvmClass("java/util/concurrent/locks/Lock")
  trait Lock:
    def lock(): Unit
    def lockInterruptibly(): Unit
    def tryLock(): scala.Boolean
    def tryLock(time: scala.Long, unit: TimeUnit): scala.Boolean
    def unlock(): Unit
    def newCondition(): Condition

  @jvmClass("java/util/concurrent/locks/Condition")
  trait Condition:
    def await(): Unit
    def awaitUninterruptibly(): Unit
    def awaitNanos(nanosTimeout: scala.Long): scala.Long
    def await(time: scala.Long, unit: TimeUnit): scala.Boolean
    def signal(): Unit
    def signalAll(): Unit

  // A lock a thread holds as many times as it took it, as the JDK's; waiting threads enter in order.
  // Under `teq interp` the lock is the layer's, keyed by `sync`; JavaScript's one thread counts its
  // holds.
  @jvmClass("java/util/concurrent/locks/ReentrantLock")
  class ReentrantLock(fair: scala.Boolean) extends Lock, java.io.Serializable:
    def this() = this(false)
    private val sync = new Object
    private var holds = 0
    private var conditions = 0

    def lock(): Unit = if java.lang.threadsRun() then lockAcquire(sync, false, -1L) else holds += 1
    def lockInterruptibly(): Unit =
      if Thread.interrupted() then throw new InterruptedException()
      if java.lang.threadsRun() then
        if lockAcquire(sync, true, -1L) == 2 then throw new InterruptedException()
      else holds += 1
    def tryLock(): scala.Boolean =
      if java.lang.threadsRun() then lockAcquire(sync, false, 0L) == 0
      else
        holds += 1
        true
    def tryLock(timeout: scala.Long, unit: TimeUnit): scala.Boolean =
      if Thread.interrupted() then throw new InterruptedException()
      if java.lang.threadsRun() then
        lockAcquire(sync, true, Math.max(unit.toNanos(timeout), 0L)) match
          case 0 => true
          case 1 => false
          case _ => throw new InterruptedException()
      else
        holds += 1
        true
    def unlock(): Unit =
      if java.lang.threadsRun() then lockRelease(sync)
      else if holds == 0 then throw new IllegalMonitorStateException()
      else holds -= 1
    def newCondition(): Condition =
      conditions += 1
      new ConditionObject(this, sync, conditions)
    def getHoldCount(): Int = if java.lang.threadsRun() then lockHolds(sync) else holds
    def isHeldByCurrentThread(): scala.Boolean = getHoldCount() > 0
    def isLocked(): scala.Boolean = if java.lang.threadsRun() then lockState(sync, false) != 0 else holds > 0
    final def isFair(): scala.Boolean = fair
    final def hasQueuedThreads(): scala.Boolean = getQueueLength() > 0
    final def getQueueLength(): Int = if java.lang.threadsRun() then lockState(sync, true) else 0
    override def toString: String = super.toString + (if isLocked() then "[Locked]" else "[Unlocked]")

  // A condition of a `ReentrantLock`: its waiters release the lock whole, wait for a signal, an
  // interrupt or the time, and take the lock back with their holds.
  private[locks] class ConditionObject(lock: ReentrantLock, sync: Object, id: Int) extends Condition, java.io.Serializable:
    private def held(): Unit = if !lock.isHeldByCurrentThread() then throw new IllegalMonitorStateException()
    def await(): Unit =
      if Thread.interrupted() then throw new InterruptedException()
      if !java.lang.threadsRun() then held()
      if conditionAwait(sync, id, -1L, true) == 2 then throw new InterruptedException()
    def awaitUninterruptibly(): Unit =
      if !java.lang.threadsRun() then held()
      conditionAwait(sync, id, -1L, false)
    def awaitNanos(nanosTimeout: scala.Long): scala.Long =
      if Thread.interrupted() then throw new InterruptedException()
      if !java.lang.threadsRun() then held()
      val deadline = System.nanoTime() + nanosTimeout
      if conditionAwait(sync, id, Math.max(nanosTimeout, 0L), true) == 2 then throw new InterruptedException()
      deadline - System.nanoTime()
    // Whether the signal won before the time, as JDK 24's `ConditionObject.await(long, TimeUnit)`
    // returns `!cancelled`, whatever taking the lock back then took.
    def await(time: scala.Long, unit: TimeUnit): scala.Boolean =
      if Thread.interrupted() then throw new InterruptedException()
      if !java.lang.threadsRun() then held()
      conditionAwait(sync, id, Math.max(unit.toNanos(time), 0L), true) match
        case 0 => true
        case 1 => false
        case _ => throw new InterruptedException()
    def signal(): Unit =
      if !java.lang.threadsRun() then held()
      conditionSignal(sync, id, false)
    def signalAll(): Unit =
      if !java.lang.threadsRun() then held()
      conditionSignal(sync, id, true)
