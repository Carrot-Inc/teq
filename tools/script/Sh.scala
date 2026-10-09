// Children of the repository's scripts, with the JDK's processes alone (no shell, no threads):
//
// - `Sh(cmd*).run()`: a finite command. Its stdout and stderr go to files (never two pipes read in
//   turn), its stdin is closed unless given text or a file, and past its deadline it is killed with
//   every process below it; after the wait its status and its outputs: the files, and as text a
//   bounded head and tail of each (`outAll`, `errAll` read the whole). A status other than 0 throws
//   `Sh.Failed`, naming the command and the end of its stderr, unless `check(false)`. A timed out
//   command has status 124, as `timeout` gives.
// - `Sh(cmd*).start()`: a resident child, its stdin and stdout pipes for a protocol (`Frames`:
//   JSON-RPC's `Content-Length` frames, counted in UTF-8 bytes, a partial one kept for the next
//   read), its stderr to a file; killed with its tree when the script ends unless `detach`.
// - `Sh.pool(n, jobs)`: at most n children at once, each job a chain of commands (`Step`), the next
//   command of a job or the next job started as soon as a child ends, each child with its deadline.
//
// A child is the script's while it runs (`Script.own`): one the scope's end or a signal finds
// running is killed with its tree, a `run`'s, a `spawn`'s or a pool's alike; a resident one too,
// unless `detach`ed.
//
// The interpreter's output is flushed before a child that inherits it starts (teq interp does so).

import java.io.File
import java.lang.ProcessBuilder.Redirect
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, Path, Paths}
import java.util.concurrent.TimeUnit

object Sh:
  def apply(cmd: String*): Cmd = Cmd(cmd.toVector)

  final class Failed(val result: Result) extends RuntimeException(
    s"${result.cmd.mkString(" ")} ${if result.timedOut then "timed out" else s"exited ${result.status}"}" +
      (if result.errTail.isEmpty then "" else ":\n" + result.errTail)
  )

  // Where the outputs of the script's children go while they run.
  private lazy val spool: Path =
    val dir = Files.createTempDirectory("teq-script-")
    Script.atExit(remove(dir))
    dir

  // A directory and what it holds, gone; nothing when there is none.
  def remove(p: Path): Unit =
    if Files.exists(p, java.nio.file.LinkOption.NOFOLLOW_LINKS) then
      if Files.isDirectory(p, java.nio.file.LinkOption.NOFOLLOW_LINKS) then
        val s = Files.list(p)
        val entries = try s.toArray.toList.map(_.asInstanceOf[Path]) finally s.close()
        entries.foreach(remove)
      Files.deleteIfExists(p)

  private var counter = 0
  private def spoolFile(suffix: String): Path =
    counter += 1
    spool.resolve(s"$counter.$suffix")

  private def nullFile: File = new File(if java.io.File.separatorChar == '\\' then "NUL" else "/dev/null")

  // A command and how it runs.
  final case class Cmd(
      args: Vector[String],
      dir: Path = null,
      env: Map[String, String] = Map.empty,
      unset: Set[String] = Set.empty,
      input: String = null,
      inputFile: Path = null,
      outFile: Path = null,
      errFile: Path = null,
      appendOut: Boolean = false,
      merged: Boolean = false,
      inherited: Boolean = false,
      timeoutMillis: Long = 0L,
      checked: Boolean = true
  ):
    def in(dir: Path): Cmd = copy(dir = dir)
    def in(dir: String): Cmd = copy(dir = Paths.get(dir))
    def withEnv(vars: (String, String)*): Cmd = copy(env = env ++ vars)
    def without(names: String*): Cmd = copy(unset = unset ++ names)
    def stdin(text: String): Cmd = copy(input = text)
    def stdinFrom(file: Path): Cmd = copy(inputFile = file)
    // stdout to the file, created or truncated (or appended to); with `mergeErr` stderr too.
    def stdout(file: Path, append: Boolean = false): Cmd = copy(outFile = file, appendOut = append)
    def stderr(file: Path): Cmd = copy(errFile = file)
    def mergeErr: Cmd = copy(merged = true)
    // The script's own stdout and stderr: what the command prints is seen as it runs.
    def inherit: Cmd = copy(inherited = true)
    def timeout(seconds: Double): Cmd = copy(timeoutMillis = (seconds * 1000).toLong)
    def check(on: Boolean): Cmd = copy(checked = on)

    private[Sh] def builder(out: Path, err: Path): ProcessBuilder =
      val pb = new ProcessBuilder(args*)
      if dir != null then pb.directory(dir.toFile)
      if env.nonEmpty || unset.nonEmpty then
        val e = pb.environment()
        unset.foreach(e.remove)
        env.foreach((k, v) => e.put(k, v))
      if input != null then
        val f = spoolFile("in")
        Files.writeString(f, input)
        pb.redirectInput(f.toFile)
      else if inputFile != null then pb.redirectInput(inputFile.toFile)
      else pb.redirectInput(nullFile)
      if inherited then
        pb.redirectOutput(Redirect.INHERIT)
        pb.redirectError(Redirect.INHERIT)
      else
        pb.redirectOutput(if appendOut then Redirect.appendTo(out.toFile) else Redirect.to(out.toFile))
        if merged then pb.redirectErrorStream(true) else pb.redirectError(err.toFile)
      pb

    // Runs the command to its end or its deadline.
    def run(): Result =
      val r = spawn().await()
      if checked && r.status != 0 then throw new Failed(r)
      r

    // The command started, for a pool or a script that waits for it later.
    def spawn(): Running =
      val out = if outFile != null then outFile else spoolFile("out")
      val err = if errFile != null then errFile else if merged then out else spoolFile("err")
      val started = System.nanoTime()
      val p = builder(out, err).start()
      new Running(this, p, out, err, started)

    // A resident child: its stdin and stdout piped, its stderr to a file.
    def start(): Proc =
      val pb = new ProcessBuilder(args*)
      if dir != null then pb.directory(dir.toFile)
      if env.nonEmpty || unset.nonEmpty then
        val e = pb.environment()
        unset.foreach(e.remove)
        env.foreach((k, v) => e.put(k, v))
      val err = if errFile != null then errFile else spoolFile("err")
      if inherited then pb.redirectError(Redirect.INHERIT) else pb.redirectError(err.toFile)
      new Proc(this, pb.start(), err)

  // A command under way, the script's until its result is taken.
  final class Running(val cmd: Cmd, val process: Process, val out: Path, val err: Path, started: Long):
    private val deadline = if cmd.timeoutMillis > 0 then started + cmd.timeoutMillis * 1000000L else Long.MaxValue
    private val ownership = Script.own(() => if process.isAlive() then kill())
    def alive: Boolean = process.isAlive()
    def overdue: Boolean = System.nanoTime() > deadline
    // Waits for the end, or for the deadline and then kills the command's tree.
    def await(): Result =
      val left = deadline - System.nanoTime()
      val ended = if deadline == Long.MaxValue then { process.waitFor(); true } else process.waitFor(left.max(0L), TimeUnit.NANOSECONDS)
      if ended then result(false) else kill()
    def kill(): Result =
      Sh.killTree(process.toHandle)
      process.waitFor()
      result(true)
    private[Sh] def result(timedOut: Boolean): Result =
      Script.disown(ownership)
      new Result(cmd.args, if timedOut then 124 else process.exitValue(), timedOut, out, err, (System.nanoTime() - started) / 1000000L, cmd.inherited)

  // A finished command: its status, its outputs (the files, the texts), its time. A text is UTF-8,
  // a malformed byte replaced; `out` and `err` read at most `TextBound` bytes of their file: all of
  // it, else its head and its tail, half the bound each, around a line saying how much was left
  // out. `outAll` and `errAll` read the whole.
  final class Result(val cmd: Vector[String], val status: Int, val timedOut: Boolean, val outFile: Path, val errFile: Path, val millis: Long, inherited: Boolean):
    def ok: Boolean = status == 0
    lazy val out: String = if inherited then "" else bounded(outFile)
    lazy val err: String = if inherited || errFile == outFile then "" else bounded(errFile)
    def outAll(): String = if inherited then "" else new String(Files.readAllBytes(outFile), UTF_8)
    def errAll(): String = if inherited || errFile == outFile then "" else new String(Files.readAllBytes(errFile), UTF_8)
    // The last lines of stderr (of stdout where the two are one) within its last `TailBound` bytes,
    // for a message.
    def errTail: String =
      if inherited then ""
      else
        val f = if errFile == outFile then outFile else errFile
        val size = Files.size(f)
        decode(trimLead(slice(f, (size - TailBound).max(0L), TailBound))).linesIterator.toVector.takeRight(20).mkString("\n")
    def lines: Vector[String] = out.linesIterator.toVector

  // The bounds of a result's texts.
  val TextBound: Int = 1 << 20
  val TailBound: Int = 16384

  // At most `TextBound` bytes of a file as text (`Result.out`).
  private def bounded(f: Path): String =
    val size = Files.size(f)
    if size <= TextBound then decode(slice(f, 0L, size.toInt))
    else
      val head = trimTail(slice(f, 0L, TextBound / 2))
      val tail = trimLead(slice(f, size - TextBound / 2, TextBound / 2))
      decode(head) + s"\n[${size - head.length - tail.length} bytes left out]\n" + decode(tail)

  // `len` bytes of a file from `from` on, fewer at its end; the file is not read before `from`.
  private def slice(f: Path, from: Long, len: Int): Array[Byte] =
    val in = Files.newInputStream(f)
    try
      var skipped = 0L
      var step = 1L
      while skipped < from && step > 0 do
        step = in.skip(from - skipped)
        skipped += step
      in.readNBytes(len)
    finally in.close()

  private def decode(b: Array[Byte]): String = new String(b, UTF_8)
  // A cut UTF-8 sequence off a slice's start (its continuation bytes) or its end.
  private def trimLead(b: Array[Byte]): Array[Byte] =
    var i = 0
    while i < b.length && i < 3 && (b(i) & 0xc0) == 0x80 do i += 1
    b.drop(i)
  private def trimTail(b: Array[Byte]): Array[Byte] =
    var back = 1
    var cut = 0
    var found = false
    while !found && back <= 3 && back <= b.length do
      val x = b(b.length - back) & 0xff
      if (x & 0xc0) != 0x80 then
        val need = if x >= 0xf0 then 4 else if x >= 0xe0 then 3 else if x >= 0xc0 then 2 else 1
        if back < need then cut = back
        found = true
      back += 1
    b.dropRight(cut)

  // Kills a process and everything below it, the descendants listed first so that none escapes by
  // being reparented.
  def killTree(h: ProcessHandle): Unit =
    val below = h.descendants().toArray.toList.map(_.asInstanceOf[ProcessHandle])
    h.destroyForcibly()
    below.foreach(_.destroyForcibly())

  // A resident child, the script's unless detached.
  final class Proc(val cmd: Cmd, val process: Process, val errFile: Path):
    var detached = false
    private val ownership = Script.own(() => kill())
    val in: java.io.OutputStream = process.getOutputStream
    val out: java.io.InputStream = process.getInputStream
    def alive: Boolean = process.isAlive()
    def pid: Long = process.pid()
    // Waits at most `seconds` for the end: its status, or None.
    def waitFor(seconds: Double): Option[Int] =
      if process.waitFor((seconds * 1000).toLong, TimeUnit.MILLISECONDS) then Some(process.exitValue()) else None
    def kill(): Unit =
      Script.disown(ownership)
      if process.isAlive() then
        Sh.killTree(process.toHandle)
        process.waitFor()
    def detach(): Unit =
      detached = true
      Script.disown(ownership)
    def err: String = Files.readString(errFile)

  // JSON-RPC's frames over a resident child's pipes: `Content-Length: <bytes>` and a blank line,
  // then the body in UTF-8; what arrives past a frame is kept for the next. `alive` says whether the
  // writer may still write: once it may not, what is left is read to the end.
  final class Frames(in: java.io.InputStream, out: java.io.OutputStream, alive: () => Boolean):
    def this(p: Proc) = this(p.out, p.in, () => p.alive)
    private var buf = new Array[Byte](0)
    private var ended = false
    def send(body: String): Unit =
      val bytes = body.getBytes(UTF_8)
      out.write(s"Content-Length: ${bytes.length}\r\n\r\n".getBytes(UTF_8))
      out.write(bytes)
      out.flush()
    // The next frame's body, or None past `seconds` (what came of the frame kept for the next
    // call) or at the end of the stream.
    def next(seconds: Double): Option[String] =
      val deadline = System.nanoTime() + (seconds * 1e9).toLong
      var result: Option[String] = None
      var done = false
      while !done do
        frame() match
          case Some(body) =>
            result = Some(body)
            done = true
          case None =>
            val n = in.available()
            if n > 0 then take(n)
            else if ended then done = true
            else if !alive() then take(65536)
            else if System.nanoTime() > deadline then done = true
            else Thread.sleep(1)
      result
    private def take(n: Int): Unit =
      val chunk = new Array[Byte](n)
      val k = in.read(chunk, 0, n)
      if k < 0 then ended = true
      else buf = buf ++ chunk.take(k)
    // A whole frame from the buffer, which is cut past it.
    private def frame(): Option[String] =
      val headerEnd = indexOf(buf, "\r\n\r\n".getBytes(UTF_8))
      if headerEnd < 0 then None
      else
        val header = new String(buf, 0, headerEnd, UTF_8)
        val length = header.linesIterator.collectFirst { case l if l.toLowerCase.startsWith("content-length:") => l.substring(15).trim.toInt }.getOrElse(0)
        val start = headerEnd + 4
        if buf.length < start + length then None
        else
          val body = new String(buf, start, length, UTF_8)
          buf = buf.drop(start + length)
          Some(body)
    private def indexOf(hay: Array[Byte], needle: Array[Byte]): Int =
      var i = 0
      var found = -1
      while found < 0 && i + needle.length <= hay.length do
        var j = 0
        while j < needle.length && hay(i + j) == needle(j) do j += 1
        if j == needle.length then found = i
        i += 1
      found

  // A job of a pool: a command and what follows its end, or the job's end.
  sealed trait Step
  final case class Then(cmd: Cmd, next: Result => Step) extends Step
  case object End extends Step

  // Runs the jobs with at most `slots` children at once: each job's first step when a slot is free,
  // a job's next command as its child ends (in the order the children end), a child past its
  // deadline killed with its tree (status 124). No threads: the children are polled.
  def pool[A](slots: Int, jobs: Iterable[A])(first: A => Step): Unit =
    val waiting = jobs.iterator
    val running = scala.collection.mutable.ArrayBuffer.empty[(Running, Result => Step)]
    def advance(step: Step): Unit = step match
      case Then(cmd, next) => running += ((cmd.spawn(), next))
      case End => ()
    while waiting.hasNext || running.nonEmpty do
      while running.length < slots.max(1) && waiting.hasNext do advance(first(waiting.next()))
      var i = 0
      var ended = false
      while i < running.length do
        val (r, next) = running(i)
        if !r.alive || r.overdue then
          running.remove(i)
          val result = if r.alive then r.kill() else r.result(false)
          advance(next(result))
          ended = true
        else i += 1
      if !ended && running.nonEmpty then Thread.sleep(2)

  // A program on the PATH, as a shell finds it (on Windows, with the PATHEXT extensions).
  def which(name: String): Option[Path] =
    val path = System.getenv("PATH")
    if path == null then None
    else
      val exts = if java.io.File.separatorChar == '\\' then Option(System.getenv("PATHEXT")).getOrElse(".EXE;.CMD;.BAT").split(';').toList.map(_.toLowerCase) else List("")
      path.split(java.io.File.pathSeparator).iterator.filter(_.nonEmpty).flatMap(d => exts.map(e => Paths.get(d, name + e))).find(p => Files.isRegularFile(p) && Files.isExecutable(p))
