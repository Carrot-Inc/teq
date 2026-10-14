//> using file sbt.scala
// Airstream's source generators as teq runs them in the place of sbt's (overlay/AirstreamTeq.scala): the
// build's own project/Generate*.scala, which the command names after this script, run with the parameters
// build.sbt's `Compile / sourceGenerators` and `Test / sourceGenerators` give them, so that they write the
// files sbt's tasks write into the checked-in source directories. From the build's root:
//
//   teq interp bench/airstream/generators.scala project/Generate*.scala -- <compile|test> [dir]
//
// `dir`, the directory a TeqCommand is given to write sources into, stays empty: the outputs are the
// checked-in files, which the configuration's source directories hold.
import java.io.File
import java.nio.charset.StandardCharsets
import java.nio.file.Files

@main def airstreamGenerators(args: String*): Unit =
  val (from, to) = (2, 22) // build.sbt's generateTupleCombinatorsFrom and generateTupleCombinatorsTo
  args.headOption match
    case Some("compile") =>
      val sourceDir = new File("src/main")
      GenerateTupleStreams(n => s"TupleStream$n", "TupleStreams.scala", sourceDir, from, to).run
      GenerateTupleSignals(n => s"TupleSignal$n", "TupleSignals.scala", sourceDir, from, to).run
      GenerateCombineStreamOps("CombineStreamOps", sourceDir, from, to).run
      GenerateCombineSignalOps("CombineSignalOps", sourceDir, from, to).run
      GenerateCombineStreamObjectOps("CombineStreamObjectOps", sourceDir, from, to).run
      GenerateCombineSignalObjectOps("CombineSignalObjectOps", sourceDir, from, to).run
    case Some("test") =>
      val testSourceDir = new File("src/test")
      GenerateCombineSignalsTest("CombineSignalsSpec", testSourceDir, from, to).run
      GenerateCombineStreamsTest("CombineStreamsSpec", testSourceDir, from, to).run
    case _ =>
      System.err.println("usage: generators.scala project/Generate*.scala -- <compile|test> [dir]")
      System.exit(2)

// project/SourceGenerator.scala, the generators' base, with its text kept in a builder and written at the end:
// the build's prints into a `PrintStream` over a `FileOutputStream`, a constructor teq's java.io does not have
// (its `PrintStream` is the console's). The lines and their ends are println's, the line separator's.
abstract class SourceGenerator(outputFile: File):
  private val text = new StringBuilder
  private val indentStep = "  "
  private var currentIndent = ""

  /** Override this to implement the generator. */
  protected def apply(): Unit

  /** Call this to run the generator. */
  final def run: List[File] =
    apply()
    outputFile.getParentFile.mkdirs()
    Files.write(outputFile.toPath, text.toString.getBytes(StandardCharsets.UTF_8))
    List(outputFile)

  protected def enter(prefix: String, suffix: String = "")(inside: => Unit): Unit =
    line(prefix)
    val originalIndent = currentIndent
    currentIndent = currentIndent + indentStep
    inside
    currentIndent = originalIndent
    if suffix.nonEmpty then line(suffix)

  protected def line(str: String): Unit =
    text.append(currentIndent).append(str).append(System.lineSeparator)

  protected def line(): Unit =
    text.append(System.lineSeparator)

  protected def tupleType(size: Int, prefix: String = "T", suffix: String = "", separator: String = ", "): String =
    (1 to size).map(i => s"${prefix}${i}${suffix}").mkString(separator)

  protected def tupleAccess(size: Int, varName: String): String =
    (1 to size).map(i => s"${varName}._${i}").mkString(", ")
