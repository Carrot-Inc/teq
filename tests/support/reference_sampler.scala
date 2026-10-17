//> using file ../../tools/script
//> using file ../../bench/reference/reference.scala
//> using file ../../bench/reference/records.scala
// The peak memory the reference verbs sample (tests/reference-test-verb.sh): a node the invocation starts, given its
// token as a verb's command is, which holds a 256 MiB buffer and names no teq binary, counted in the sample, which
// prints the child's resident set and the peak.
import java.nio.file.Paths

@main def referenceSampler(argv: String*): Unit = Script.run {
  val Seq(out) = argv: @unchecked
  val (r, peak) = Reference.sampled(Paths.get(out), 20)(token =>
    Sh("node", "-e", "global.buf = Buffer.alloc(256 * 1024 * 1024, 1); console.log(Math.round(process.memoryUsage().rss / 1024)); setTimeout(() => {}, 3000)").withEnv(token).timeout(10).run())
  println(s"${r.out.trim} ${peak.getOrElse(0L)}")
}
