//> using platform jvm
// The thread-safe lazy val's waiting (dotty's mkThreadSafeDef over scala-library's LazyVals: Evaluating, Waiting,
// NullValue): a second reader waits for the first's initialiser and gets its value; an initialiser that throws
// releases the waiter, which then computes the value itself; a computed null is computed once; an interrupted waiter
// leaves with InterruptedException while the first reader finishes; a reader of a lazy val of a broad type (Any,
// AnyRef, Serializable, a type parameter, a union) never gets a control state for the value; and independent lazy
// vals, of one instance or of two, initialise at once without waiting on each other. Deadlines bound every wait.
import java.util.concurrent.{ConcurrentLinkedQueue, CountDownLatch, TimeUnit}
import java.util.concurrent.atomic.{AtomicInteger, AtomicReference}

// The run's acceptance: a worker's exception, a value other than the one expected, a wait or a state poll that passes
// its deadline, and a worker still running at the end each fail the run, which prints what failed and exits 1.
object Check:
  private val failures = new ConcurrentLinkedQueue[String]
  private val workers = new ConcurrentLinkedQueue[Thread]
  def fail(what: String): Unit = { failures.add(what); () }
  def that(ok: Boolean, what: String): Unit = if !ok then fail(what)
  /** Prints `line`, the run failing where it is not `expected`. */
  def show(line: String, expected: String): Unit =
    println(line)
    that(line == expected, s"expected `$expected`")
  /** A thread running `body`, joined by `done`; its exception fails the run. */
  def worker(name: String)(body: => Unit): Thread =
    val t = new Thread(() => try body catch case e: Throwable => fail(s"$name threw $e"))
    t.setName(name)
    workers.add(t)
    t.start()
    t
  def await(latch: CountDownLatch, what: String): Unit = that(latch.await(5, TimeUnit.SECONDS), s"deadline passed: $what")
  /** Waits until `reached` holds, the run failing where it does not within the deadline. */
  def poll(what: String)(reached: => Boolean): Unit =
    val end = System.nanoTime + 5000000000L
    while !reached && System.nanoTime < end do Thread.sleep(1)
    that(reached, s"not reached: $what")
  def join(t: Thread): Unit =
    t.join(10000)
    that(!t.isAlive, s"${t.getName} still running")
  /** Joins every worker, then exits 1 where anything failed. */
  def done(): Unit =
    val it = workers.iterator()
    while it.hasNext do join(it.next())
    if !failures.isEmpty then
      val f = failures.iterator()
      while f.hasNext do println("FAILED: " + f.next())
      System.exit(1)

trait Gated:
  val calls = new AtomicInteger
  val entered = new CountDownLatch(1)
  val release = new CountDownLatch(1)
  def gated(): Int =
    val n = calls.incrementAndGet()
    entered.countDown()
    if !release.await(3, TimeUnit.SECONDS) then throw new Exception("deadline")
    n

class Modes(fail: Boolean, nil: Boolean) extends Gated:
  lazy val value: String =
    val n = gated()
    if fail && n == 1 then throw new Exception("first")
    if nil then null else "ok"

class AnyHolder extends Gated:
  lazy val value: Any = { gated(); "any" }
class RefHolder extends Gated:
  lazy val value: AnyRef = { gated(); "ref" }
case class Tag(name: String)
class SerHolder extends Gated:
  lazy val value: java.io.Serializable = { gated(); Tag("serializable") }
class GenHolder[T](v: T) extends Gated:
  lazy val value: T = { gated(); v }
class UnionHolder extends Gated:
  lazy val value: Int | String = { gated(); "union" }

class Pair:
  val aEntered = new CountDownLatch(1)
  val bEntered = new CountDownLatch(1)
  lazy val a: String = { aEntered.countDown(); if bEntered.await(3, TimeUnit.SECONDS) then "a" else "a alone" }
  lazy val b: String = { bEntered.countDown(); if aEntered.await(3, TimeUnit.SECONDS) then "b" else "b alone" }

object Twins:
  val entered = Array(new CountDownLatch(1), new CountDownLatch(1))
class Twin(i: Int):
  lazy val v: String = { Twins.entered(i).countDown(); if Twins.entered(1 - i).await(3, TimeUnit.SECONDS) then s"twin $i" else s"twin $i alone" }

object Main:
  def waiting(t: Thread): Boolean = t.getState == Thread.State.WAITING || t.getState == Thread.State.TIMED_WAITING

  def contend(label: String, g: Gated, read: () => Any, expected: Any, line: String): Unit =
    val first = new AtomicReference[Any]("none")
    val second = new AtomicReference[Any]("none")
    def reading(into: AtomicReference[Any]): Unit =
      try into.set(read()) catch case e: Exception => into.set("threw " + e.getMessage)
    val owner = Check.worker(s"$label owner")(reading(first))
    Check.await(g.entered, s"$label: the initialiser entered")
    val waiter = Check.worker(s"$label waiter")(reading(second))
    Check.poll(s"$label: the second reader waiting")(waiting(waiter))
    g.release.countDown()
    Check.join(owner)
    Check.join(waiter)
    def shown(v: Any) = if v == expected then "the value" else if v == null then "null" else v.toString.takeWhile(_ != '@')
    Check.show(s"$label: calls=${g.calls.get} first=${shown(first.get)} second=${shown(second.get)} finished=${!owner.isAlive && !waiter.isAlive}", s"$label: $line")

  def main(args: Array[String]): Unit =
    val once = "calls=1 first=the value second=the value finished=true"
    val ok = new Modes(false, false)
    contend("contended", ok, () => ok.value, "ok", once)
    val failing = new Modes(true, false)
    contend("first initialiser throws", failing, () => failing.value, "ok", "calls=2 first=threw first second=the value finished=true")
    val nil = new Modes(false, true)
    contend("contended null", nil, () => nil.value, null, once)
    Check.show(s"null computed once: ${nil.value == null} calls=${nil.calls.get}", "null computed once: true calls=1")

    val anyH = new AnyHolder
    contend("Any", anyH, () => anyH.value, "any", once)
    val refH = new RefHolder
    contend("AnyRef", refH, () => refH.value, "ref", once)
    val serH = new SerHolder
    contend("Serializable", serH, () => serH.value, Tag("serializable"), once)
    val genH = new GenHolder[String]("generic")
    contend("type parameter", genH, () => genH.value, "generic", once)
    val unionH = new UnionHolder
    contend("union", unionH, () => unionH.value, "union", once)

    val interrupted = new Modes(false, false)
    val interruptions = new AtomicInteger
    val owner = Check.worker("interrupted owner") { interrupted.value; () }
    Check.await(interrupted.entered, "interrupted: the initialiser entered")
    val waiter = Check.worker("interrupted waiter") {
      try { interrupted.value; () }
      catch case _: InterruptedException => interruptions.incrementAndGet(); ()
    }
    Check.poll("interrupted: the second reader waiting")(waiting(waiter))
    waiter.interrupt()
    Check.join(waiter)
    interrupted.release.countDown()
    Check.join(owner)
    Check.show(s"interrupted waiter: calls=${interrupted.calls.get} interrupted=${interruptions.get} value=${interrupted.value} finished=${!waiter.isAlive && !owner.isAlive}", "interrupted waiter: calls=1 interrupted=1 value=ok finished=true")

    val pair = new Pair
    val ra = new AtomicReference[String]
    val rb = new AtomicReference[String]
    val ta = Check.worker("pair a")(ra.set(pair.a))
    val tb = Check.worker("pair b")(rb.set(pair.b))
    Check.join(ta)
    Check.join(tb)
    Check.show(s"two lazy vals of one instance: ${ra.get} ${rb.get}", "two lazy vals of one instance: a b")
    val twins = Array(new Twin(0), new Twin(1))
    val r0 = new AtomicReference[String]
    val r1 = new AtomicReference[String]
    val t0 = Check.worker("twin 0")(r0.set(twins(0).v))
    val t1 = Check.worker("twin 1")(r1.set(twins(1).v))
    Check.join(t0)
    Check.join(t1)
    Check.show(s"one lazy val of two instances: ${r0.get}, ${r1.get}", "one lazy val of two instances: twin 0, twin 1")
    Check.done()
