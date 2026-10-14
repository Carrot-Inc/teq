import sbt._
import sbt.Keys._
import dev.teq.sbt.TeqPlugin
import dev.teq.sbt.TeqPlugin.autoImport._

// Laminar's DOM definitions as teq runs them: build.sbt's `preload`, which sbt runs when it loads the build (its
// `onLoad`) and no generator list names, writes them with project/DomDefsGenerator.scala over Scala DOM Types, a
// library of the meta-build. teq runs the same file under `teq interp` (bench/laminar/domdefs.scala, staged in the
// checkout) over the library's Scala 3 jar at the version the meta-build names (sbt-buildinfo's
// metaProject.BuildInfo), which the load retrieves into target/domtypes, writing the checked-in files of
// src/main/scala/com/raquo/laminar: those whose header says they are generated, the command's outputs. The root
// project `laminar` alone has the command; websiteJS and website, the documentation's, are not built by teq's runs.
// Scala 2.12, the meta-build of sbt 1.
object LaminarTeq extends AutoPlugin {
  override def requires = TeqPlugin
  override def trigger = allRequirements

  private def scripts: File = file(sys.props.getOrElse("teq.laminar.scripts", sys.error("teq.laminar.scripts names no directory")))

  // The generator's script and the file it reads (`//> using file metaProject.scala`), staged inside the checkout
  // where the build loads, so that the command names them by the checkout's paths and teq's record of the generator
  // hashes them (an edit to either runs it again): teq hashes the files of the repository a command names and its
  // inputs. A file is written only where its bytes differ, keeping its time otherwise.
  private val staged = "target/teq-scripts"
  private val closure = Seq("domdefs.scala", "metaProject.scala")
  private def stage(root: File): Unit =
    for (name <- closure) {
      val (from, to) = (scripts / name, root / staged / name)
      if (!to.isFile || !java.util.Arrays.equals(IO.readBytes(from), IO.readBytes(to))) IO.copyFile(from, to)
    }

  private val domTypes = "com.raquo" % "domtypes_3" % metaProject.BuildInfo.scalaDomTypesVersion
  private val domTypesJar = s"target/domtypes/domtypes_3-${metaProject.BuildInfo.scalaDomTypesVersion}.jar"
  private val generatedRoot = "src/main/scala/com/raquo/laminar"
  private val header = "#NOTE: GENERATED CODE"

  val teqLaminarDomTypes = taskKey[File]("Scala DOM Types' Scala 3 jar, which teq's run of the DOM definitions' generator reads")

  // The files DomDefsGenerator writes, by the header it gives them.
  private def generated(root: File): Seq[String] =
    ((root / generatedRoot) ** "*.scala").get().filter(f => IO.readLines(f).take(12).exists(_.contains(header)))
      .map(f => IO.relativize(root, f).get).sorted

  private def domDefs(root: File): TeqCommand =
    TeqCommand(
      Seq("teq", "interp", s"$staged/domdefs.scala", "project/DomDefsGenerator.scala",
        "project/project/ProjectVersions.scala", "--classpath", domTypesJar, "--"),
      inputs = Seq("project/DomDefsGenerator.scala", "project/project/ProjectVersions.scala") ++ closure.map(name => s"$staged/$name"),
      outputs = generated(root))

  override def globalSettings = Seq(
    // The jar retrieved as sbt loads the build, before its commands run, as the build's own `preload` is.
    Global / onLoad := (Global / onLoad).value.andThen { state => stage(state.baseDir); "laminar/teqLaminarDomTypes" :: state },
  )

  override def projectSettings = Seq(
    teqLaminarDomTypes := {
      val root = (LocalRootProject / baseDirectory).value
      val retrieved = dependencyResolution.value.retrieve(domTypes.intransitive(), None, root / "target" / "domtypes-retrieve", streams.value.log)
        .fold(w => throw w.resolveException, identity).filter(_.getName.endsWith(".jar"))
      val jar = root / domTypesJar
      IO.copyFile(retrieved.headOption.getOrElse(sys.error(s"$domTypes resolves to no jar")), jar)
      jar
    },
    Compile / teqGenerators ++= (if (thisProject.value.id == "laminar") Seq(domDefs((LocalRootProject / baseDirectory).value)) else Nil),
    Compile / teqReplacesSbtGenerators := thisProject.value.id == "laminar",
    Test / teqReplacesSbtGenerators := thisProject.value.id == "laminar",
  )
}
