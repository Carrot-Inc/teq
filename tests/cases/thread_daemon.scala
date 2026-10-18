//> using platform jvm
// The program ends when main has returned and no thread but daemons runs: a daemon thread sleeping
// forever is left running, a non-daemon thread started by main outlives it and ends the program
// when it ends; `setDaemon` on a started thread is refused.
object Main:
  def main(args: Array[String]): Unit =
    val d = new Thread(() => while true do Thread.sleep(1000))
    d.setDaemon(true)
    d.start()
    println("daemon " + d.isDaemon + ", alive " + d.isAlive)
    try d.setDaemon(false)
    catch case e: IllegalThreadStateException => println("setDaemon on a started thread: " + e.getClass.getName)
    val late = new Thread(() => { Thread.sleep(100); println("the last non-daemon thread ends") })
    println("inherits daemon: " + late.isDaemon)
    late.start()
    println("main ends")
