// Object's wait and notify named without arguments where a function is expected: eta-expanded as any member is,
// each call of the function the monitor's (dotty's etaExpand of the selected method).
object Main:
  def main(args: Array[String]): Unit =
    val g = new Object
    val waiting: Long => Unit = g.wait
    val notifying: () => Unit = g.notify
    g.synchronized { notifying(); waiting(1L) }
    println("returned")
