//> using file ../../tools/script
//> using file ../../bench/reference/reference.scala
// The peak memory the reference verbs sample (tests/reference-test-verb.sh): a node the script starts, which holds a
// 256 MiB buffer and names no teq binary, counted in the sample, which prints the child's resident set and the peak.
import java.nio.file.Paths

@main def referenceSampler(argv: String*): Unit = Script.run {
  val Seq(out) = argv: @unchecked
  val (r, peak) = Reference.sampled("/no/teq/binary", Paths.get(out), 20)(
    Sh("node", "-e", "global.buf = Buffer.alloc(256 * 1024 * 1024, 1); console.log(Math.round(process.memoryUsage().rss / 1024)); setTimeout(() => {}, 3000)").timeout(10).run())
  println(s"${r.out.trim} ${peak.getOrElse(0L)}")
}
