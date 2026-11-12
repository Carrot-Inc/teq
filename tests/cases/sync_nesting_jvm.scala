//> using platform jvm
// The monitors a thread holds through nested and reentrant `synchronized` blocks: the same lock still held after an
// inner block on it ends, two locks nested, and blocks inside a lazy val initialiser, a constructor, a lambda, a
// by-name argument and a local def, none held once the outermost block has ended.
class Guarded(lock: AnyRef):
  val held: Boolean = lock.synchronized(Thread.holdsLock(lock))

object Main:
  val a = new Object
  val b = new Object
  lazy val initialised: Boolean = a.synchronized(Thread.holdsLock(a))
  def byName[T](x: => T): T = x
  def holds: String = s"a=${Thread.holdsLock(a)} b=${Thread.holdsLock(b)}"

  def main(args: Array[String]): Unit =
    val reentrant = a.synchronized {
      val inner = a.synchronized(Thread.holdsLock(a))
      s"inner=$inner after inner=${Thread.holdsLock(a)}"
    }
    println(s"reentrant $reentrant then $holds")
    val two = a.synchronized(b.synchronized(holds))
    println(s"two locks $two then $holds")
    println(s"lazy initialiser $initialised then $holds")
    println(s"constructor ${new Guarded(b).held} then $holds")
    val f = () => a.synchronized(holds)
    println(s"lambda ${f()} then $holds")
    println(s"by-name ${byName(b.synchronized(holds))} then $holds")
    def local(n: Int): String = a.synchronized(if n == 0 then holds else local(n - 1))
    println(s"local def ${local(3)} then $holds")
