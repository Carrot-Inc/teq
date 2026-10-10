package dev.teq.sbt

import java.io.File

import sbt.MessageOnlyException
import sbt.util.Logger

/** One resident `teq compiler watch` process per link directory (`teqLinkJS`, a Scala.js project's links
  * under the toggle), kept for the sbt session: a build sent to it costs an incremental build, not
  * a fresh process start. teq builds once on start, before reading stdin (`docs/TARGETS.md`,
  * "Watch mode"), and that build answers the request that started the process; every build after
  * that is asked for with a `build` line and answered with one JSON line. A process holds a
  * build's shape (inputs, excludes, classpath, flags, the binary) as it was started with: a
  * build whose command differs, after a `set` or a changed dependency, whose binary was rebuilt
  * since, or whose class path entries changed in place (a jar republished at the same path)
  * replaces it; a process that died is replaced too. A build error is reported the same way and the process stays: the
  * next build is incremental against it, as in any watch. The processes end when the build is
  * unloaded (`reload`, exit: `onUnload`) and with the JVM (a shutdown hook each), and once it has
  * been idle for its bound with no watch holding it (`Life`).
  * The registry is keyed by the directory's canonical path, and a process that is stopped or
  * replaced is waited for, so that a directory never has two. */
object Resident {
  /** How long a link's process stays: it ends `idle` after its last build unless a watch holds
    * it. A build that a watch asked for brings the question whether that watch is still under
    * way (`watch`): the hold lasts as long as the answer is yes, whichever task the watch is of,
    * and lapses by itself. */
  final case class Life(idle: scala.concurrent.duration.FiniteDuration, watch: Option[() => Boolean])

  private val sessions = new java.util.concurrent.ConcurrentHashMap[File, Session]()
  @volatile private var closed = false

  /** Called with the directory of a process that ended without a successor. */
  @volatile var onStopped: File => Unit = (_: File) => ()

  def running(out: File): Boolean = {
    val session = sessions.get(out.getCanonicalFile)
    session != null && session.alive
  }

  /** Sends the output directory's process a plain `build` (the resident checks every file's
    * modification time itself, which finds an edit, a file added or removed), starting one first
    * if there is none yet, the last one died or its command or binary changed; the answer of a
    * process started for this request is the build it made on start. A process that dies under
    * the request is replaced and the build sent once more. A process starts as `teq @<file>`, the
    * rest of `command` written to its file at `argsPlace` (`ArgsFile`); the session keeps the
    * command itself. */
  def build(out: File, command: Seq[String], argsPlace: ArgsFile.Place, root: File, log: Logger, life: Life): String = {
    val key = out.getCanonicalFile
    def current(): Session = {
      val stamp = (binaryStamp(command), classpathStamp(command))
      val session = sessions.compute(
        key,
        (_, old) =>
          if (closed) { if (old != null) old.stop(); null }
          else if (old != null && old.alive && old.command == command && old.stamp == stamp) old
          else { if (old != null) old.stop(); Session.start(command, argsPlace, stamp, root, log) },
      )
      if (session == null) throw new MessageOnlyException("teq: the build is being unloaded")
      session
    }
    def built(): String = {
      val session = current()
      try session.build()
      finally session.used(life.watch, life.idle, () => stopIdle(key, session))
    }
    try built()
    catch { case _: DeadSession => built() }
  }

  /** Ends the output directory's process and waits for it, so that the next build starts one
    * afresh. */
  def stop(out: File): Unit = {
    val key = out.getCanonicalFile
    val session = sessions.remove(key)
    if (session != null) {
      session.stop()
      onStopped(key)
    }
  }

  /** The session's timer has run: the session ends when it is idle and no watch holds it,
    * and is looked at again after another bound while one does. */
  private def stopIdle(key: File, session: Session): Unit =
    if (!session.idle) session.lookAgain(() => stopIdle(key, session))
    else if (sessions.remove(key, session)) {
      session.stop()
      onStopped(key)
    }

  /** Stops every process when the build is unloaded (`reload`, a `set`, exit); a build under
    * way, woken by its process dying, finds the registry closed and starts no replacement. The
    * registry opens again once the processes are gone, since a `set` reloads the build and the
    * builds after it start their residents afresh. */
  def stopAll(): Unit = {
    closed = true
    val stopped = new java.util.ArrayList[File]()
    sessions.forEach { (key, session) =>
      session.stop()
      stopped.add(key)
    }
    sessions.clear()
    stopped.forEach(key => onStopped(key))
    closed = false
  }

  /** The size and modification time of the binary a command runs: a binary rebuilt at the same
    * path is another compiler. */
  private def binaryStamp(command: Seq[String]): (Long, Long) = {
    val binary = new File(command.head)
    if (binary.isFile) (binary.length, binary.lastModified) else (0L, 0L)
  }

  /** The entries of the command's `--classpath`, each with its size and modification time (a
    * directory's over its class files): the resident holds what it read from them, so an entry
    * replaced in place is a reason to start afresh. */
  def classpathStamp(command: Seq[String]): Seq[(String, Long, Long)] =
    command.sliding(2).collectFirst { case Seq("--classpath", entries) => entries }.toSeq
      .flatMap(_.split(File.pathSeparator).filter(_.nonEmpty))
      .map(entry => fileStamp(new File(entry)))

  def fileStamp(f: File): (String, Long, Long) = stampOver(f, _.endsWith(".class"))


  private def stampOver(f: File, counts: String => Boolean): (String, Long, Long) =
    if (f.isDirectory) {
      val files = sbt.io.Path.allSubpaths(f).map(_._1).filter(x => x.isFile && counts(x.getName)).toSeq
      (f.getPath, files.map(_.length).sum, files.map(_.lastModified).foldLeft(0L)(_ max _))
    }
    else if (f.isFile) (f.getPath, f.length, f.lastModified)
    else (f.getPath, 0L, 0L)

  /** The process died between two builds or under one, or teq exited on its own (a crash, a
    * `quit` sent by another session sharing the JVM). */
  private final class DeadSession extends Exception

  private final class Session(val command: Seq[String], val stamp: ((Long, Long), Seq[(String, Long, Long)]), process: java.lang.Process, log: Logger) {
    private val stdin = new java.io.OutputStreamWriter(process.getOutputStream, "UTF-8")
    private val stdout = new java.io.BufferedReader(new java.io.InputStreamReader(process.getInputStream, "UTF-8"))
    private var answered = false
    @volatile private var dead = false
    private val hook = new Thread(() => stop())
    java.lang.Runtime.getRuntime.addShutdownHook(hook)

    def alive: Boolean = !dead && process.isAlive

    // A link's life: the time of its last build, its idle bound, and the watch that holds it,
    // as the question whether that watch is under way. Read and written under the session's
    // monitor, which a build holds while it runs; the timer's thread is the session's own and
    // ends with it.
    private var heldBy: Option[() => Boolean] = None
    private var lastUsed = System.nanoTime
    private var idleAfter = 0L
    @volatile private var timer: java.util.Timer = null
    private var pending: java.util.TimerTask = null

    /** After a build: the session is looked at once `idle` has passed. */
    def used(watch: Option[() => Boolean], idle: scala.concurrent.duration.FiniteDuration, look: () => Unit): Unit = synchronized {
      if (watch.isDefined) heldBy = watch
      idleAfter = idle.toNanos
      lookAgain(look)
    }

    def lookAgain(look: () => Unit): Unit = synchronized {
      if (alive) {
        if (timer == null) timer = new java.util.Timer("teq-resident-idle", true)
        if (pending != null) pending.cancel()
        pending = new java.util.TimerTask { def run(): Unit = look() }
        timer.schedule(pending, idleAfter / 1000000 + 50)
      }
    }

    /** Idle for its bound, and held by no watch that is still under way. */
    def idle: Boolean = synchronized {
      val held = heldBy.exists(underWay => try underWay() catch { case _: Exception => false })
      if (!held) heldBy = None
      !held && System.nanoTime - lastUsed >= idleAfter
    }

    private def cancelTimer(): Unit = {
      val running = timer
      timer = null
      if (running != null) running.cancel()
    }

    /** Reads the answer of the build the process made on start, for the request that comes
      * first, else sends `build` and reads its answer. Two sbt clients of one resident process
      * (two terminals running `~fastLinkJS`) are serialised. */
    def build(): String = synchronized {
      if (!alive) throw new DeadSession
      try {
        if (answered) {
          stdin.write("build\n")
          stdin.flush()
        }
        val line = read()
        answered = true
        lastUsed = System.nanoTime
        line
      }
      catch {
        case _: java.io.IOException => died()
        case e: InterruptedException =>
          // The task was cancelled under the request: the answer will arrive unread, so the
          // process goes, and the next build starts afresh.
          stop()
          throw e
      }
    }

    private def died(): Nothing = {
      dead = true
      throw new DeadSession
    }

    private def read(): String = {
      val line = stdout.readLine()
      if (line == null) died()
      line
    }

    /** Ends the process and returns once it is gone: `quit`, which an idle process obeys at
      * once, then the kill, for one in the middle of a build. */
    def stop(): Unit = {
      dead = true
      cancelTimer()
      try
        if (process.isAlive) {
          stdin.write("quit\n")
          stdin.flush()
        }
      catch { case _: java.io.IOException => () }
      try
        if (!process.waitFor(500, java.util.concurrent.TimeUnit.MILLISECONDS)) {
          process.destroyForcibly()
          process.waitFor(StopBound.toMillis, java.util.concurrent.TimeUnit.MILLISECONDS)
        }
      catch { case _: InterruptedException => process.destroyForcibly() }
      try java.lang.Runtime.getRuntime.removeShutdownHook(hook)
      catch { case _: IllegalStateException => () } // the JVM is shutting down: this is the hook
    }
  }

  /** How long a stopped process is waited for after its kill. */
  val StopBound: scala.concurrent.duration.FiniteDuration = scala.concurrent.duration.DurationInt(5).seconds

  private object Session {
    def start(command: Seq[String], argsPlace: ArgsFile.Place, stamp: ((Long, Long), Seq[(String, Long, Long)]), root: File, log: Logger): Session = {
      log.debug(s"teq: starting ${command.mkString(" ")} (its arguments in ${ArgsFile.file(argsPlace, command.tail)})")
      val builder = new ProcessBuilder(ArgsFile.command(command, argsPlace)*)
      builder.directory(root)
      builder.redirectErrorStream(false)
      builder.redirectError(ProcessBuilder.Redirect.INHERIT)
      val process =
        try builder.start()
        catch { case e: java.io.IOException => throw new MessageOnlyException(s"teq could not be started as ${command.head} (teqBinary or TEQ overrides it): ${e.getMessage}") }
      new Session(command, stamp, process, log)
    }
  }
}
