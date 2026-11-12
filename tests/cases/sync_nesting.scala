// Nested and reentrant `synchronized` blocks, and blocks inside a lazy val initialiser, a constructor, a lambda, a
// by-name argument and a local def, each giving its value. tests/cases/sync_nesting_jvm checks the monitors held at
// each point on the JVM.
class Guarded(lock: AnyRef):
  val made: String = lock.synchronized("made in a constructor")

object Main:
  val a = new Object
  val b = new Object
  lazy val initialised: String = a.synchronized("lazy " + a.synchronized("reentrant"))
  def byName[T](x: => T): T = x

  def main(args: Array[String]): Unit =
    println(a.synchronized(a.synchronized(1) + a.synchronized(2)))
    println(a.synchronized(b.synchronized(a.synchronized("a b a"))))
    println(initialised)
    println(new Guarded(a).made)
    val f = (x: Int) => a.synchronized(x * 2)
    println(f(21))
    println(byName(b.synchronized("by name")))
    def local(n: Int): Int = a.synchronized(if n == 0 then 0 else n + local(n - 1))
    println(local(4))
