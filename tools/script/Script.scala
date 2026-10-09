// The life of a repository script: its body runs in `Script.run { ... }` and ends by returning or
// by `Script.exit(status)` (an exception the scope catches: the body never calls `sys.exit`). The
// handlers `Script.atExit` registers run once, the last registered first, when the scope ends,
// however it ends (a return, an exit, an exception), and when the script is stopped by SIGINT or
// SIGTERM (a JDK shutdown hook, which `teq interp` runs at its exit): they release what the script
// holds (a remote claim, an edited file put back, a child's tree killed). Before them the children
// the script owns (`own`, each child of `Sh` while it runs) are killed with their trees. A child
// started on purpose to outlive the script (`Sh.Proc.detach`) is left alone.

import scala.util.control.ControlThrowable

object Script:
  final class Exit(val status: Int) extends ControlThrowable

  private val handlers = scala.collection.mutable.ArrayBuffer.empty[() => Unit]
  private var running = false
  // The owned children's kills, by key, in the order they were owned.
  private val owned = scala.collection.mutable.LinkedHashMap.empty[Long, () => Unit]
  private var ownedKeys = 0L

  def atExit(handler: => Unit): Unit = handlers.synchronized(handlers += (() => handler))

  // Owns a child while it runs: `kill` ends it with its tree if the scope ends first. The key
  // disowns it (`disown`) once it has ended or is detached.
  def own(kill: () => Unit): Long = handlers.synchronized {
    ownedKeys += 1
    owned(ownedKeys) = kill
    ownedKeys
  }
  def disown(key: Long): Unit = handlers.synchronized(owned.remove(key))

  def exit(status: Int): Nothing = throw new Exit(status)

  // The owned children killed, the last owned first, then the handlers, each once, the last first;
  // one that fails is reported and the others still run.
  private def release(): Unit =
    val todo = handlers.synchronized {
      val all = owned.values.toList.reverse ++ handlers.toList.reverse
      owned.clear()
      handlers.clear()
      all
    }
    todo.foreach { h =>
      try h()
      catch case e: Exception => System.err.println("cleanup: " + e)
    }

  // Runs the body, then the handlers, then ends the process with the body's status: 0, the status
  // given to `exit`, or 1 for an exception, reported as the JVM reports one.
  def run(body: => Unit): Unit =
    if running then body
    else
      running = true
      val hook = new Thread(() => release())
      Runtime.getRuntime.addShutdownHook(hook)
      val status =
        try
          body
          0
        catch
          case e: Exit => e.status
          case e: Throwable =>
            System.err.println("Exception in thread \"main\" " + e)
            1
      release()
      Runtime.getRuntime.removeShutdownHook(hook)
      System.out.flush()
      System.exit(status)
