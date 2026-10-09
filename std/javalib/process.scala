// The processes of the JDK under `teq interp` (src/interp/process.rs): `ProcessBuilder` starts a
// child with the arguments as given, no shell between, in the working directory and with the
// environment it names, each standard stream a pipe, the interpreter's own, the null device or a
// file; `Process` waits for it, reads its status, its pipes, and ends it; `ProcessHandle` reads the
// system's process table. A child that ends is reaped whether or not its `Process` is kept, and none
// is ended because the program dropped it or ended. JavaScript has no processes. On the JVM the
// classes are the JDK's.
package java.lang:

  @js("$fail(\"UnsupportedOperationException\", \"processes are not available on JavaScript\")")
  def processStart(command: Array[String], dir: String, env: Array[String], redirects: Array[String], merge: scala.Boolean): Array[scala.Long]
  @js("$fail(\"UnsupportedOperationException\", \"processes are not available on JavaScript\")")
  def processPoll(child: scala.Long): scala.Long
  @js("$fail(\"UnsupportedOperationException\", \"processes are not available on JavaScript\")")
  def processWait(child: scala.Long, nanos: scala.Long): scala.Long
  @js("$fail(\"UnsupportedOperationException\", \"processes are not available on JavaScript\")")
  def processEnd(child: scala.Long, force: scala.Boolean): Unit
  @js("$fail(\"UnsupportedOperationException\", \"processes are not available on JavaScript\")")
  def processCurrent(): scala.Long
  @js("$fail(\"UnsupportedOperationException\", \"processes are not available on JavaScript\")")
  def processAlive(pid: scala.Long): scala.Boolean
  @js("$fail(\"UnsupportedOperationException\", \"processes are not available on JavaScript\")")
  def processSignal(pid: scala.Long, force: scala.Boolean): scala.Boolean
  @js("$fail(\"UnsupportedOperationException\", \"processes are not available on JavaScript\")")
  def processTable(): Array[scala.Long]
  @js("$fail(\"UnsupportedOperationException\", \"processes are not available on JavaScript\")")
  def processInfo(pid: scala.Long): Array[Any]

  @javaDefined
  @jvmClass("java/lang/ProcessBuilder")
  final class ProcessBuilder(private var commandList: java.util.List[String]):
    if commandList == null then throw new NullPointerException()
    private var workDir: java.io.File = null
    private var env: java.util.Map[String, String] = null
    private val redirects = Array[ProcessBuilder.Redirect](ProcessBuilder.Redirect.PIPE, ProcessBuilder.Redirect.PIPE, ProcessBuilder.Redirect.PIPE)
    private var mergeErrors = false

    def this(command: String*) = this(ProcessBuilder.listOf(command))

    def command(): java.util.List[String] = commandList
    def command(command: java.util.List[String]): ProcessBuilder =
      if command == null then throw new NullPointerException()
      commandList = command
      this
    def command(command: String*): ProcessBuilder =
      commandList = ProcessBuilder.listOf(command)
      this
    def directory(): java.io.File = workDir
    def directory(dir: java.io.File): ProcessBuilder =
      workDir = dir
      this
    // The interpreter's environment, copied when first asked for: what the child gets, a removal
    // removing a variable from the child's alone. Its names are as the system gives them and keys
    // match by case on Windows too, as the JDK's copy (`ProcessEnvironment.environment()`), so a
    // name of another case is a variable of its own (process.rs: `windows_block`).
    def environment(): java.util.Map[String, String] =
      if env == null then env = new java.util.HashMap[String, String](System.getenv())
      env
    def redirectInput(source: ProcessBuilder.Redirect): ProcessBuilder =
      if source.`type`() == ProcessBuilder.Redirect.Type.WRITE || source.`type`() == ProcessBuilder.Redirect.Type.APPEND then
        throw new IllegalArgumentException("Redirect invalid for reading: " + source)
      redirects(0) = source
      this
    def redirectOutput(destination: ProcessBuilder.Redirect): ProcessBuilder =
      if destination.`type`() == ProcessBuilder.Redirect.Type.READ then
        throw new IllegalArgumentException("Redirect invalid for writing: " + destination)
      redirects(1) = destination
      this
    def redirectError(destination: ProcessBuilder.Redirect): ProcessBuilder =
      if destination.`type`() == ProcessBuilder.Redirect.Type.READ then
        throw new IllegalArgumentException("Redirect invalid for writing: " + destination)
      redirects(2) = destination
      this
    def redirectInput(file: java.io.File): ProcessBuilder = redirectInput(ProcessBuilder.Redirect.from(file))
    def redirectOutput(file: java.io.File): ProcessBuilder = redirectOutput(ProcessBuilder.Redirect.to(file))
    def redirectError(file: java.io.File): ProcessBuilder = redirectError(ProcessBuilder.Redirect.to(file))
    def redirectInput(): ProcessBuilder.Redirect = redirects(0)
    def redirectOutput(): ProcessBuilder.Redirect = redirects(1)
    def redirectError(): ProcessBuilder.Redirect = redirects(2)
    def inheritIO(): ProcessBuilder =
      redirects(0) = ProcessBuilder.Redirect.INHERIT
      redirects(1) = ProcessBuilder.Redirect.INHERIT
      redirects(2) = ProcessBuilder.Redirect.INHERIT
      this
    def redirectErrorStream(): scala.Boolean = mergeErrors
    def redirectErrorStream(redirectErrorStream: scala.Boolean): ProcessBuilder =
      mergeErrors = redirectErrorStream
      this

    // The child, or the JDK's `IOException` ("Cannot run program ...").
    def start(): Process =
      val n = commandList.size()
      val cmd = new Array[String](n)
      var i = 0
      while i < n do
        cmd(i) = commandList.get(i)
        if cmd(i) == null then throw new NullPointerException()
        i += 1
      if n == 0 then throw new ArrayIndexOutOfBoundsException("Index 0 out of bounds for length 0")
      var vars: Array[String] = null
      if env != null then
        vars = new Array[String](env.size() * 2)
        val it = env.entrySet().iterator()
        var k = 0
        while it.hasNext do
          val e = it.next()
          vars(k) = e.getKey
          vars(k + 1) = e.getValue
          k += 2
      val codes = new Array[String](3)
      var r = 0
      while r < 3 do
        codes(r) = redirects(r).code
        r += 1
      val dir = if workDir == null then null else workDir.getPath
      val ids = processStart(cmd, dir, vars, codes, mergeErrors)
      new ChildProcess(ids(0), ids(1), ids(2).toInt, ids(3).toInt, ids(4).toInt)

  @javaDefined
  @jvmClass("java/lang/ProcessBuilder")
  object ProcessBuilder:
    private def listOf(command: Seq[String]): java.util.List[String] =
      val list = new java.util.ArrayList[String]()
      command.foreach(c => list.add(c))
      list

    // Where a standard stream of the child goes: a pipe, the interpreter's own, the null device,
    // or a file read, written or appended to. Two redirects of one file are equal.
    @javaDefined
    @jvmClass("java/lang/ProcessBuilder$Redirect")
    abstract class Redirect:
      def `type`(): Redirect.Type
      def file(): java.io.File = null
      private[lang] def code: String =
        val t = `type`()
        if this eq Redirect.DISCARD then "discard"
        else if t == Redirect.Type.PIPE then "pipe"
        else if t == Redirect.Type.INHERIT then "inherit"
        else if t == Redirect.Type.READ then "read:" + file().getPath
        else if t == Redirect.Type.WRITE then "write:" + file().getPath
        else "append:" + file().getPath
      override def toString: String = `type`().toString
      override def equals(other: Any): scala.Boolean = other match
        case r: Redirect if r eq this => true
        case r: Redirect => file() != null && r.`type`() == `type`() && file() == r.file()
        case _ => false
      override def hashCode(): Int = if file() == null then super.hashCode() else file().hashCode()

    @javaDefined
    @jvmClass("java/lang/ProcessBuilder$Redirect")
    object Redirect:
      enum Type:
        case PIPE, INHERIT, READ, WRITE, APPEND
      private final class Of(kind: Type, target: java.io.File, label: String) extends Redirect:
        def `type`(): Type = kind
        override def file(): java.io.File = target
        override def toString: String = if label == null then kind.toString else label + " file \"" + target + "\""
      val PIPE: Redirect = new Of(Type.PIPE, null, null)
      val INHERIT: Redirect = new Of(Type.INHERIT, null, null)
      val DISCARD: Redirect = new Of(Type.WRITE, new java.io.File(if java.nio.file.windowsPaths then "NUL" else "/dev/null"), null)
      def from(file: java.io.File): Redirect =
        if file == null then throw new NullPointerException()
        new Of(Type.READ, file, "redirect to read from")
      def to(file: java.io.File): Redirect =
        if file == null then throw new NullPointerException()
        new Of(Type.WRITE, file, "redirect to write to")
      def appendTo(file: java.io.File): Redirect =
        if file == null then throw new NullPointerException()
        new Of(Type.APPEND, file, "redirect to append to")

    // The streams of a stream the child does not pipe: nothing to read, and a write refused.
    @javaDefined
    @jvmClass("java/lang/ProcessBuilder$NullInputStream")
    final class NullInputStream private[lang] () extends java.io.InputStream:
      def read(): Int = -1
      override def available(): Int = 0

    @javaDefined
    @jvmClass("java/lang/ProcessBuilder$NullOutputStream")
    final class NullOutputStream private[lang] () extends java.io.OutputStream:
      def write(b: Int): Unit = throw new java.io.IOException("Stream closed")

  @javaDefined
  @jvmClass("java/lang/Process")
  abstract class Process:
    def getOutputStream(): java.io.OutputStream
    def getInputStream(): java.io.InputStream
    def getErrorStream(): java.io.InputStream
    def waitFor(): Int
    def waitFor(timeout: scala.Long, unit: java.util.concurrent.TimeUnit): scala.Boolean
    def exitValue(): Int
    def destroy(): Unit
    def destroyForcibly(): Process =
      destroy()
      this
    def supportsNormalTermination(): scala.Boolean
    def isAlive(): scala.Boolean
    def pid(): scala.Long = toHandle().pid()
    def toHandle(): ProcessHandle
    def info(): ProcessHandle.Info = toHandle().info()
    def children(): java.util.stream.Stream[ProcessHandle] = toHandle().children()
    def descendants(): java.util.stream.Stream[ProcessHandle] = toHandle().descendants()
    // The pipes as text, UTF-8 (the JDK's `native.encoding` on the systems teq runs on).
    def inputReader(): java.io.BufferedReader = new java.io.BufferedReader(new java.io.InputStreamReader(getInputStream()))
    def errorReader(): java.io.BufferedReader = new java.io.BufferedReader(new java.io.InputStreamReader(getErrorStream()))
    def outputWriter(): java.io.BufferedWriter = new java.io.BufferedWriter(new java.io.OutputStreamWriter(getOutputStream()))

  // A child `ProcessBuilder.start` started: its index among the interpreter's children, its pid
  // and its pipes' streams (-1 for a stream that is no pipe).
  @javaDefined
  private[lang] final class ChildProcess(child: scala.Long, val processId: scala.Long, stdin: Int, stdout: Int, stderr: Int) extends Process:
    private val in: java.io.OutputStream = if stdin < 0 then new ProcessBuilder.NullOutputStream() else new java.io.NativeOutputStream(stdin)
    private val out: java.io.InputStream = if stdout < 0 then new ProcessBuilder.NullInputStream() else new java.io.NativeInputStream(stdout)
    private val err: java.io.InputStream = if stderr < 0 then new ProcessBuilder.NullInputStream() else new java.io.NativeInputStream(stderr)
    def getOutputStream(): java.io.OutputStream = in
    def getInputStream(): java.io.InputStream = out
    def getErrorStream(): java.io.InputStream = err
    def waitFor(): Int = processWait(child, -1L).toInt
    def waitFor(timeout: scala.Long, unit: java.util.concurrent.TimeUnit): scala.Boolean =
      val nanos = unit.toNanos(timeout)
      processWait(child, if nanos < 0L then 0L else nanos) != scala.Long.MinValue
    def exitValue(): Int =
      val status = processPoll(child)
      if status == scala.Long.MinValue then throw new IllegalThreadStateException("process hasn't exited")
      status.toInt
    // SIGTERM on Unix, SIGKILL when forcibly; the status is kept for a later wait.
    def destroy(): Unit = processEnd(child, false)
    override def destroyForcibly(): Process =
      processEnd(child, true)
      this
    def supportsNormalTermination(): scala.Boolean = !java.nio.file.windowsPaths
    def isAlive(): scala.Boolean = processPoll(child) == scala.Long.MinValue
    override def pid(): scala.Long = processId
    def toHandle(): ProcessHandle = new ProcessHandle.Of(processId)
    override def toString: String =
      val status = processPoll(child)
      "Process[pid=" + processId + ", exitValue=" + (if status == scala.Long.MinValue then "\"not exited\"" else status.toString) + "]"

  @javaDefined
  @jvmClass("java/lang/ProcessHandle")
  trait ProcessHandle extends Comparable[ProcessHandle]:
    def pid(): scala.Long
    def parent(): java.util.Optional[ProcessHandle]
    def children(): java.util.stream.Stream[ProcessHandle]
    def descendants(): java.util.stream.Stream[ProcessHandle]
    def info(): ProcessHandle.Info
    def supportsNormalTermination(): scala.Boolean
    def destroy(): scala.Boolean
    def destroyForcibly(): scala.Boolean
    def isAlive(): scala.Boolean

  @javaDefined
  @jvmClass("java/lang/ProcessHandle")
  object ProcessHandle:
    def current(): ProcessHandle = new Of(processCurrent())
    def of(pid: scala.Long): java.util.Optional[ProcessHandle] =
      if pid == processCurrent() || processAlive(pid) then java.util.Optional.of(new Of(pid)) else java.util.Optional.empty()
    // Every process the system lists.
    def allProcesses(): java.util.stream.Stream[ProcessHandle] =
      val table = processTable()
      val list = new java.util.ArrayList[ProcessHandle]()
      var i = 0
      while i + 1 < table.length do
        list.add(new Of(table(i)))
        i += 2
      list.stream()

    // What the system tells of a process: its executable, its arguments past the first, and the
    // two joined; on Windows the executable alone, as the JDK reads there.
    @javaDefined
    @jvmClass("java/lang/ProcessHandle$Info")
    trait Info:
      def command(): java.util.Optional[String]
      def commandLine(): java.util.Optional[String]
      def arguments(): java.util.Optional[Array[String]]

    private final class InfoOf(exe: String, args: Array[String]) extends Info:
      def command(): java.util.Optional[String] = java.util.Optional.ofNullable(exe)
      def commandLine(): java.util.Optional[String] =
        if exe != null && args != null then java.util.Optional.of(if args.isEmpty then exe else exe + " " + args.mkString(" "))
        else java.util.Optional.empty()
      def arguments(): java.util.Optional[Array[String]] = java.util.Optional.ofNullable(args)
      override def toString: String =
        "[" + (if exe != null then "user: Optional.empty, cmd: " + exe else "user: Optional.empty") +
          (if args != null then ", args: [" + args.mkString(", ") + "]" else "") + "]"

    private[lang] final class Of(id: scala.Long) extends ProcessHandle:
      def pid(): scala.Long = id
      def parent(): java.util.Optional[ProcessHandle] =
        val table = processTable()
        var i = 0
        var found: ProcessHandle = null
        while i + 1 < table.length && found == null do
          if table(i) == id && table(i + 1) > 0 then found = new Of(table(i + 1))
          i += 2
        java.util.Optional.ofNullable(found)
      // The processes whose parent this one is; and theirs, breadth first.
      def children(): java.util.stream.Stream[ProcessHandle] = below(false)
      def descendants(): java.util.stream.Stream[ProcessHandle] = below(true)
      private def below(all: scala.Boolean): java.util.stream.Stream[ProcessHandle] =
        val table = processTable()
        val list = new java.util.ArrayList[ProcessHandle]()
        var parents = List(id)
        while parents.nonEmpty do
          var next: List[scala.Long] = Nil
          var i = 0
          while i + 1 < table.length do
            if parents.contains(table(i + 1)) && table(i) != table(i + 1) then
              list.add(new Of(table(i)))
              next = table(i) :: next
            i += 2
          parents = if all then next.reverse else Nil
        list.stream()
      def info(): Info =
        val parts = processInfo(id)
        val exe = parts(0).asInstanceOf[String]
        val args =
          if parts(1).asInstanceOf[scala.Boolean] then
            val a = new Array[String](parts.length - 2)
            var i = 0
            while i < a.length do
              a(i) = parts(i + 2).asInstanceOf[String]
              i += 1
            a
          else null
        new InfoOf(exe, args)
      def supportsNormalTermination(): scala.Boolean = !java.nio.file.windowsPaths
      def destroy(): scala.Boolean = end(false)
      def destroyForcibly(): scala.Boolean = end(true)
      private def end(force: scala.Boolean): scala.Boolean =
        if id == processCurrent() then throw new IllegalStateException("destroy of current process not allowed")
        processSignal(id, force)
      def isAlive(): scala.Boolean = processAlive(id)
      def compareTo(other: ProcessHandle): Int = java.lang.Long.compare(id, other.pid())
      override def equals(other: Any): scala.Boolean = other match
        case h: ProcessHandle => h.pid() == id
        case _ => false
      override def hashCode(): Int = java.lang.Long.hashCode(id)
      override def toString: String = id.toString
