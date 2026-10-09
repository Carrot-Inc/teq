package dev.teq.sbt

import java.io.File
import java.nio.channels.{FileChannel, FileLock, OverlappingFileLockException}
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.StandardOpenOption.{CREATE, READ, WRITE}
import java.util.concurrent.{ConcurrentHashMap, Semaphore, TimeUnit}

import sbt.MessageOnlyException
import sbt.internal.teq.Evaluating
import sbt.io.IO
import sbt.util.Logger

/** A linker's output directory has one writer. Two files beside it say who: `<dir>.lock`, which
  * the sbt process that writes the directory holds locked (an operating system lock, so it goes
  * with the process) for as long as it has a build under way or a resident there, and
  * `<dir>.backend`, which names what wrote the files in it, as a class directory's marker does.
  * Inside one sbt process a directory has one writer at a time: a link of teq's for as long as
  * it builds and writes (`writing`), the Scala.js linker from the task before its link to the
  * one after it (`claim`, `done`). A link waits for a link of teq's to end, and fails where
  * the Scala.js linker has the directory, naming it: the linker's link is a task still to
  * run, which a task that waits for it may keep from running. Another process waits for the
  * lock, `teqLinkWait` at most, and fails naming the holder. */
object Directory:
  import scala.concurrent.duration.FiniteDuration

  private final class Hold(val channel: FileChannel, val lock: FileLock)

  /** The linker's turn: the evaluation it links in, and the task. */
  private final case class Claim(evaluation: AnyRef, by: String)

  // A turn is given back by another thread than took it where a task takes it and a later
  // one ends it, so it is a permit and no lock of a thread's.
  private val turns = new ConcurrentHashMap[File, Semaphore]()
  private val claims = new ConcurrentHashMap[File, Claim]()
  private val holds = new ConcurrentHashMap[File, Hold]()

  private def turn(dir: File): Semaphore = turns.computeIfAbsent(dir, _ => new Semaphore(1))

  private def beside(dir: File, suffix: String): File = new File(dir.getParentFile, dir.getName + suffix)

  private def take(dir: File): Unit =
    while !turn(dir).tryAcquire(100, TimeUnit.MILLISECONDS) do
      val claim = claims.get(dir)
      if claim != null then
        throw new MessageOnlyException(
          s"teq: ${claim.by} has $dir in this command until its link has ended: " +
            "another link into the directory belongs in a command of its own")

  /** Runs `body` as the directory's writer. The lock is kept afterwards while a resident of
    * this process writes there, and given back when it ends (`release`). */
  def writing[A](directory: File, wait: FiniteDuration, log: Logger)(body: File => A): A =
    val dir = directory.getCanonicalFile
    take(dir)
    try
      hold(dir, wait, log)
      try body(dir)
      finally if !Resident.running(dir) then drop(dir)
    finally turn(dir).release()

  /** Takes the directory for a writer that runs after `body`, in a task of its own (the
    * Scala.js linker): `by` has it until `done`. */
  def claim(directory: File, by: String, wait: FiniteDuration, log: Logger)(body: File => Unit): Unit =
    val dir = directory.getCanonicalFile
    take(dir)
    try
      hold(dir, wait, log)
      body(dir)
      claims.put(dir, Claim(Evaluating.now, by))
    finally if !claims.containsKey(dir) then give(dir)

  /** The end of the link `by` claimed the directory for, whether it wrote or failed. */
  def done(directory: File, by: String): Unit =
    val dir = directory.getCanonicalFile
    if claims.remove(dir, Claim(Evaluating.now, by)) then give(dir)

  /** The end of an evaluation of tasks, cancelled or not (`Cancelling.atEnd`): sbt runs none of
    * a cancelled evaluation's remaining tasks, the one that would end its claim among them, so
    * a claim of the evaluation that is still there is given back here, the lock with it. */
  def evaluationEnded(): Unit =
    val ended = Evaluating.now
    claims.forEach { (dir, claim) =>
      if (claim.evaluation eq ended) && claims.remove(dir, claim) then give(dir)
    }

  private def give(dir: File): Unit =
    try if !Resident.running(dir) then drop(dir)
    finally turn(dir).release()

  /** Gives the directory back once its resident has ended; a writer under way keeps it, and
    * gives it back itself. */
  def release(directory: File): Unit =
    val dir = directory.getCanonicalFile
    if turn(dir).tryAcquire() then give(dir)

  /** Gives every directory back: the build is unloaded, and its residents are stopped. */
  def releaseAll(): Unit = holds.keySet.forEach(dir => drop(dir))

  private def drop(dir: File): Unit =
    val held = holds.remove(dir)
    if held != null then
      try held.lock.release()
      catch case _: java.io.IOException => ()
      held.channel.close()

  private def hold(dir: File, wait: FiniteDuration, log: Logger): Unit =
    if !holds.containsKey(dir) then
      val file = beside(dir, ".lock")
      IO.createDirectory(file.getParentFile)
      val channel = FileChannel.open(file.toPath, CREATE, READ, WRITE)
      val deadline = System.nanoTime + wait.toNanos
      var lock: FileLock = null
      var told = false
      try
        while lock == null do
          lock =
            try channel.tryLock()
            catch case _: OverlappingFileLockException => null
          if lock == null then
            if System.nanoTime > deadline then
              throw new MessageOnlyException(
                s"teq: $dir is written by another sbt process (${holder(channel)}), which holds ${file.getName}: " +
                  "end its watch, or wait for its resident to end")
            if !told then
              log.info(s"teq: waiting for ${holder(channel)} to give up $dir")
              told = true
            Thread.sleep(100)
        channel.truncate(0)
        channel.write(java.nio.ByteBuffer.wrap(s"pid ${ProcessHandle.current.pid}\n".getBytes(UTF_8)), 0)
        holds.put(dir, new Hold(channel, lock))
      finally if lock == null then channel.close()

  private def holder(channel: FileChannel): String =
    val bytes = java.nio.ByteBuffer.allocate(64)
    try
      channel.read(bytes, 0)
      new String(bytes.array, 0, bytes.position, UTF_8).trim match
        case "" => "its pid unknown"
        case pid => pid
    catch case _: java.io.IOException => "its pid unknown"

  /** What wrote the directory: `None` for the Scala.js linker (no marker), else teq's identity. */
  def writer(dir: File): Option[String] =
    val marker = beside(dir, ".backend")
    if marker.isFile then Some(IO.read(marker)) else None

  def mark(dir: File, identity: String): Unit =
    val marker = beside(dir, ".backend")
    if !marker.isFile || IO.read(marker) != identity then IO.write(marker, identity)

  /** Before a link by `identity` (`None`: the Scala.js linker): what another writer left in the
    * directory goes, files and marker, with the resident that wrote them, so that neither
    * compiler's files stay for the other to serve. True when the directory changed hands. */
  def takeOver(dir: File, identity: Option[String], log: Logger): Boolean =
    val was = writer(dir)
    if was == identity then false
    else
      val left = Option(dir.listFiles).toSeq.flatten
      if left.nonEmpty then
        log.info(s"teq: $dir was written by ${was.getOrElse("the Scala.js linker")}; ${identity.fold("the Scala.js linker")(_ => "teq")} writes it afresh")
      Resident.stop(dir)
      IO.delete(left)
      IO.delete(beside(dir, ".backend"))
      true
