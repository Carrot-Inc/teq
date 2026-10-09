//> using platform jvm
// os: unix
//> using file ../../tools/script
// The library's children are the script's while they run (`Script.own`): when the scope ends, the
// children it finds running are killed with their trees, a `spawn`'s never awaited and a pool's
// left by a failing step alike, before the handlers run; a finished one is the script's no longer;
// a detached resident child is left alone. A handler registered first runs last and sees them.
object Main:
  def main(args: Array[String]): Unit = Script.run {
    def running(): Long = ProcessHandle.current().children().filter(_.isAlive).count()
    var detached: Sh.Proc = null
    Script.atExit {
      println("running after the scope: " + running())
      println("the detached child alive: " + detached.alive)
      detached.process.destroyForcibly()
      detached.process.waitFor()
    }
    val finished = Sh("true").run()
    println("finished: " + finished.status)
    val spawned = Sh("sleep", "30").spawn()
    try
      Sh.pool(2, List(1, 2)) { i =>
        if i == 1 then Sh.Then(Sh("sleep", "30"), _ => Sh.End)
        else Sh.Then(Sh("true"), _ => throw new IllegalStateException("a step failed"))
      }
    catch case e: IllegalStateException => println("pool: " + e.getMessage)
    detached = Sh("sleep", "30").start()
    detached.detach()
    println("running before the end: " + running())
    println("the spawned child alive: " + spawned.alive)
  }
