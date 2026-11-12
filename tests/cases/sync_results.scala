// The value of a `synchronized` block of every kind: Int, Long and Double (the wide slots), Boolean, Unit, a
// reference, null, and Nothing (a throw), alone and beside operands on the stack (dotty's genSynchronized keeps the
// result in a local across the monitorexit).
object Main:
  val lock = new Object

  def main(args: Array[String]): Unit =
    val i: Int = lock.synchronized(41 + 1)
    val l: Long = lock.synchronized(1L << 40)
    val d: Double = lock.synchronized(1.5)
    val z: Boolean = lock.synchronized(i == 42)
    val u: Unit = lock.synchronized(println("unit body"))
    val s: String = lock.synchronized("ref")
    val n: String = lock.synchronized(null)
    println(s"$i $l $d $z $u $s $n")
    val thrown =
      try lock.synchronized(throw new IllegalStateException("thrown"))
      catch case e: IllegalStateException => e.getMessage
    println(thrown)
    println(1L + lock.synchronized(2L) + lock.synchronized(3.5))
    println(lock.synchronized(lock.synchronized(7)) * 6)
    val parts = List(1, 2, 3).map(x => lock.synchronized(x * 10))
    println(parts)
