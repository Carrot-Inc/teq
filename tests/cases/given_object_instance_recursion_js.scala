//> using platform js
// A given object of a class instance whose body reads it again: Scala.js makes it as a lazy val
// without a latch and recurses (the JVM's thread waits on its own latch, dotty's
// `LazyVals.transformMemberDefThreadSafe`, and never ends: no JVM case).
trait T { val self: T }
class C:
  def helper: T = u
  given u: T with
    val self: T = helper
@main def run(): Unit =
  val c = new C
  val r = try { if c.u.self eq c.u then "instance" else "other" } catch case e: Throwable => "overflow"
  println(r)
