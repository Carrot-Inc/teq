//> using file ../../tools/script
//> using file ../reference/reference.scala
//> using file ../reference/scalajs.scala
// Airstream (github.com/raquo/Airstream), the FRP library of Laminar, as one of teq's public reference projects
// for the JavaScript target: its pinned release checked out, built and tested under scalac 3.8.4 by its own sbt
// build (the oracle), exported with sbt-teq through the overlay, then compiled and tested by teq against the
// oracle's results (bench/reference/scalajs.scala has the verbs). Run from the repository's root:
//
//   ./teq interp bench/airstream/airstream.scala -- <verb> <checkout> <out> [options]
//
// with the verbs checkout, oracle, export, compile and test, and generators.
//
// The overlay declares build.sbt's seven Compile and two Test source generators as the commands teq runs
// (generators.scala over the build's own project/Generate*.scala, staged in the checkout with the file it reads),
// writing the checked-in files they write. The verb `generators` holds teq's record of them to their closure: over
// the export, `teq compile` writes the generated files; with the staged script's arity bound edited (2 to 22, then
// 2 to 21) it runs them again, TupleStreams.scala without TupleStream22; restored, again the pin's bytes.
import java.nio.file.{Files, Path}

@main def airstream(argv: String*): Unit = Script.run {
  ScalaJsReference.main("airstream", argv, Seq("generators")) { (project, _, dir, out, teq, _) =>
    staleGenerators(project, dir, out, teq)
  }
}

def staleGenerators(project: ScalaJsReference, dir: Path, out: Path, teq: String): Unit =
  project.exported(dir)
  val bin = project.binary(teq)
  val script = dir.resolve("target/teq-scripts/generators.scala")
  if !Files.isRegularFile(script) then Log.die(s"no staged script at $script: export again with the overlay")
  val streams = dir.resolve("src/main/scala/com/raquo/airstream/extensions/TupleStreams.scala")
  val (bound, edited) = ("val (from, to) = (2, 22)", "val (from, to) = (2, 21)")
  val text = Files.readString(script)
  if !text.contains(bound) then Log.die(s"$script does not say $bound")
  // The compile's own errors are not this verb's: the generators run before the projects are typed.
  def compiled(i: Int): Boolean =
    val log = out.resolve(s"generators-$i.log")
    Sh(bin, "compile", "airstream").in(dir).stdout(log).mergeErr.timeout(project.teqSeconds).check(false).run()
    Files.isRegularFile(streams) && Files.readString(streams).contains("class TupleStream22")
  def clean: String = Sh("git", "-C", dir.toString, "status", "--porcelain", "--untracked-files=no").timeout(300).run().out.trim
  Sh(bin, "stop").in(dir).timeout(120).check(false).run()
  var failed = false
  if !compiled(1) then
    project.fail("the generators did not write TupleStream22 from the staged script as exported")
    failed = true
  Files.writeString(script, text.replace(bound, edited))
  val stale = try compiled(2) finally Files.writeString(script, text)
  if stale then
    project.fail("the staged script's arity bound edited to 21, the generators did not run again: TupleStream22 stays")
    failed = true
  else project.say("the staged script's arity bound edited to 21: the generators ran again, TupleStream22 gone")
  if !compiled(3) || clean.nonEmpty then
    project.fail(s"the script restored, the generated files are not the pin's:\n$clean")
    Sh("git", "-C", dir.toString, "checkout", "--", ".").timeout(300).run()
    failed = true
  else project.say("the script restored: the generated files the pin's again")
  Sh(bin, "stop").in(dir).timeout(120).check(false).run()
  Script.exit(if failed then 1 else 0)
