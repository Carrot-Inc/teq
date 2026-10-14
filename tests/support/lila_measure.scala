//> using file ../../bench/lila/lila.scala
// The review's probe of the verbs' measurement (tests/lila-test-verb.sh): an empty workload beside an
// unrelated process, one that names the runner's class and holds 64 MiB, measures nothing; a workload
// whose command carries the invocation's token measures what that command's process holds.
import java.nio.file.{Files, Paths}

@main def lilaMeasure(argv: String*): Unit = Script.run {
  val Seq(out) = argv: @unchecked
  val dir = Paths.get(out)
  Files.createDirectories(dir)
  val hold = "import time; b = b'x' * (64 << 20); time.sleep(4)"
  val unrelated = Sh("python3", "-c", hold, "dev.teq.runner.Runner").timeout(30).check(false).spawn()
  val (_, empty) = Reference.sampled(dir, 30)(_ => Thread.sleep(2200))
  unrelated.kill()
  val (_, own) = Reference.sampled(dir, 30)(token => Sh("python3", "-c", hold).withEnv(token).timeout(30).check(false).run())
  println(s"lila-measure: an empty workload beside an unrelated process ${empty.getOrElse(-1L)} KB, the workload's own process ${own.getOrElse(-1L)} KB")
  Script.exit(if empty.contains(0L) && own.exists(_ >= 60000L) then 0 else 1)
}
