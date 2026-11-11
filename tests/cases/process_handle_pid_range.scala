//> using platform jvm
// os: unix
// `ProcessHandle.of` takes its pid as the JDK's `pid_t`, an int: a long past the int range wraps, and
// one that wraps to 0 or below names no process, so its `Optional` is empty. As an int,
// `4294967295` is -1, which `kill` reads as every process the user may signal; `4294967297` is
// process 1, which every system has, under its own long.
@main def run(): Unit =
  for pid <- List(4294967295L, 2147483648L, -1L, 0L, 4294967296L, 4294967297L) do
    println(s"$pid ${ProcessHandle.of(pid).map(_.pid()).map(_.toString).orElse("none")}")
  val current = ProcessHandle.current().pid()
  println(s"current ${ProcessHandle.of(current).map(_.pid() == current).orElse(false)}")
  // A handle of a long that wraps to this process: `pid()` the long, its parent and children
  // those of the process the JDK's natives name by the truncated pid (`parent0`,
  // `getProcessPids0`); its descendants none, the JDK's walk comparing the long itself.
  val wrapped = ProcessHandle.of(current + 4294967296L)
  println(s"wrapped ${wrapped.map(_.pid() == current + 4294967296L).orElse(false)}")
  val parent = ProcessHandle.current().parent().map(_.pid())
  println(s"wrapped parent ${wrapped.map(_.parent().map(_.pid()) == parent).orElse(false)}")
  val child = new ProcessBuilder("sleep", "30").start()
  println(s"wrapped child ${wrapped.map(_.children().anyMatch(_.pid() == child.pid())).orElse(false)}")
  println(s"wrapped descendant ${wrapped.map(_.descendants().anyMatch(_.pid() == child.pid())).orElse(false)}")
  child.destroy()
  child.waitFor()
