import sbt._
import sbt.Keys._
import dev.teq.sbt.TeqPlugin
import dev.teq.sbt.TeqPlugin.autoImport._

// Airstream's generators as teq runs them, in the place of the sbt tasks that write the same files
// (sbt-teq's `teqReplacesSbtGenerators`): build.sbt's seven `Compile / sourceGenerators` and two
// `Test / sourceGenerators`, the build's own project/Generate*.scala run by bench/airstream/generators.scala
// (staged in the checkout) under `teq interp`, writing the checked-in files they write. Scala 2.12, the meta-build
// of sbt 1.
object AirstreamTeq extends AutoPlugin {
  override def requires = TeqPlugin
  override def trigger = allRequirements

  private def scripts: File = file(sys.props.getOrElse("teq.airstream.scripts", sys.error("teq.airstream.scripts names no directory")))

  // The generators' script and the file it reads (`//> using file sbt.scala`), staged inside the checkout where the
  // build loads, so that the command names them by the checkout's paths and teq's record of the generator hashes
  // them (an edit to either runs it again): teq hashes the files of the repository a command names and its inputs.
  // A file is written only where its bytes differ, keeping its time otherwise.
  private val staged = "target/teq-scripts"
  private val closure = Seq("generators.scala", "sbt.scala")
  private def stage(root: File): Unit =
    for (name <- closure) {
      val (from, to) = (scripts / name, root / staged / name)
      if (!to.isFile || !java.util.Arrays.equals(IO.readBytes(from), IO.readBytes(to))) IO.copyFile(from, to)
    }

  // Each generator's class, by its file under project/, and the file it writes.
  private val main = Seq(
    "GenerateTupleStreams" -> "src/main/scala/com/raquo/airstream/extensions/TupleStreams.scala",
    "GenerateTupleSignals" -> "src/main/scala/com/raquo/airstream/extensions/TupleSignals.scala",
    "GenerateCombineStreamOps" -> "src/main/scala/com/raquo/airstream/combine/generated/CombineStreamOps.scala",
    "GenerateCombineSignalOps" -> "src/main/scala/com/raquo/airstream/combine/generated/CombineSignalOps.scala",
    "GenerateCombineStreamObjectOps" -> "src/main/scala/com/raquo/airstream/combine/generated/CombineStreamObjectOps.scala",
    "GenerateCombineSignalObjectOps" -> "src/main/scala/com/raquo/airstream/combine/generated/CombineSignalObjectOps.scala")
  private val test = Seq(
    "GenerateCombineSignalsTest" -> "src/test/scala/com/raquo/airstream/combine/generated/CombineSignalsSpec.scala",
    "GenerateCombineStreamsTest" -> "src/test/scala/com/raquo/airstream/combine/generated/CombineStreamsSpec.scala")

  // The script compiles every generator, so each command names them all; `inputs` are those files and the staged
  // script's closure.
  private val sources = (main ++ test).map { case (name, _) => s"project/$name.scala" }

  private def generators(configuration: String, written: Seq[(String, String)]): TeqCommand =
    TeqCommand(
      Seq("teq", "interp", s"$staged/generators.scala") ++ sources ++ Seq("--", configuration),
      inputs = sources ++ closure.map(name => s"$staged/$name"),
      outputs = written.map(_._2))

  override def globalSettings = Seq(
    Global / onLoad := (Global / onLoad).value.andThen { state => stage(state.baseDir); state },
  )

  override def projectSettings = Seq(
    Compile / teqGenerators += generators("compile", main),
    Test / teqGenerators += generators("test", test),
    Compile / teqReplacesSbtGenerators := true,
    Test / teqReplacesSbtGenerators := true,
  )
}
