//> using platform jvm
// An unqualified `synchronized`, `getClass` or `notifyAll` inside a function literal is the enclosing instance's,
// whatever the literal is converted to (dotty types the name as a selection on This(cls), and the literal is a
// closure of the class until ExpandSAMs): a Java interface, a Scala trait of one method, nested, in a local def,
// passed as an argument, in the constructor, in an object; `this.synchronized`, a Scala function type and an
// anonymous class written out beside them.
import java.util.concurrent.Callable

trait Job { def go(): Unit }

class Host:
  override def toString = "Host"
  override def hashCode = 42
  def name = "the host's member"
  def run(): String = "Host.run"

  def javaInterface(): Boolean =
    var held = false
    val r: Runnable = () => synchronized { held = Thread.holdsLock(this) }
    r.run(); held
  def control(): Boolean =
    var held = false
    val r: Runnable = () => this.synchronized { held = Thread.holdsLock(this) }
    r.run(); held
  def scalaFunction(): Boolean =
    val f: () => Boolean = () => synchronized { Thread.holdsLock(this) }
    f()
  def scalaTrait(): Boolean =
    var held = false
    val j: Job = () => synchronized { held = Thread.holdsLock(this) }
    j.go(); held
  def nested(): Boolean =
    var held = false
    val r: Runnable = () => {
      val f: () => Unit = () => { val q: Runnable = () => synchronized { held = Thread.holdsLock(this) }; q.run() }
      f()
    }
    r.run(); held
  def localDef(): Boolean =
    var held = false
    def go(): Unit = { val r: Runnable = () => synchronized { held = Thread.holdsLock(this) }; r.run() }
    go(); held
  def passedAsArgument(): Boolean =
    var held = false
    val t = new Thread(() => synchronized { held = Thread.holdsLock(this) })
    t.start(); t.join(); held

  var heldInConstructor = false
  private val inConstructor: Runnable = () => synchronized { heldInConstructor = Thread.holdsLock(this) }
  inConstructor.run()

  def otherMembers(): String =
    val c: Callable[String] = () => s"$toString $hashCode ${getClass.getSimpleName} $name"
    c.call()
  def notifyInside(): String =
    val r: Runnable = () => notifyAll()
    try { this.synchronized { r.run() }; "notified" } catch case _: IllegalMonitorStateException => "IllegalMonitorStateException"
  def memberNamedAsTheInterfaces(): String =
    val c: Callable[String] = () => run()
    c.call()
  // The control the other way: in an anonymous class written out, `this` and its monitor are the object's own.
  def writtenAnonymousClass(): String =
    var held = ""
    val r = new Runnable { def run(): Unit = synchronized { held = s"its own ${Thread.holdsLock(this)}, the host's ${Thread.holdsLock(Host.this)}" } }
    r.run(); held

object Module:
  def javaInterface(): Boolean =
    var held = false
    val r: Runnable = () => synchronized { held = Thread.holdsLock(this) }
    r.run(); held

@main def run(): Unit =
  val h = new Host
  println(s"a Java interface: holds the host's monitor: ${h.javaInterface()}")
  println(s"this.synchronized (the control): ${h.control()}")
  println(s"a Scala function type: ${h.scalaFunction()}")
  println(s"a Scala trait of one method: ${h.scalaTrait()}")
  println(s"nested, a Java interface in a function in a Java interface: ${h.nested()}")
  println(s"in a local def: ${h.localDef()}")
  println(s"passed as an argument (new Thread): ${h.passedAsArgument()}")
  println(s"in the constructor: ${h.heldInConstructor}")
  println(s"in an object: ${Module.javaInterface()}")
  println(s"toString, hashCode, getClass, a member: ${h.otherMembers()}")
  println(s"notifyAll inside the host's block: ${h.notifyInside()}")
  println(s"a member named as the interface's method: ${h.memberNamedAsTheInterfaces()}")
  println(s"an anonymous class written out: ${h.writtenAnonymousClass()}")
