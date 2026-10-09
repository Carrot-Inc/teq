sbtPlugin := true
organization := "build.teq"
name := "sbt-teq"
// The plugin's own version line (plugin-version.txt, which bench/release.sh --plugin bumps and build.rs bakes into
// the compiler as the plugin it selects), apart from the compiler's: a compiler release that changes no plugin
// publishes none.
version := IO.read(baseDirectory.value / "plugin-version.txt").trim
description := "An sbt 2 plugin that compiles, links and exports a Scala build with teq"
// The pom, the signing and the staging of a release for Maven Central (project/Central.scala): the jar, its
// sources and a javadoc jar holding a README, since scaladoc gives the plugin's users nothing to read.
Central.settings
Compile / packageDoc / mappings := Central.placeholder("javadoc", Def.setting(
  s"""# build.teq:sbt-teq ${version.value}
    |
    |The sbt 2 plugin of teq, the compiler for Scala 3. It has no API documentation: https://teq.build says how
    |a build uses it, and its sources are the sources jar beside this one, and ${Central.repository} at the
    |commit `${Central.commit(baseDirectory.value)}`.
    |""".stripMargin)).value
// For a Scala.js project's link tasks (TeqScalaJSPlugin), which the plugin answers from teq's output.
addSbtPlugin("org.scala-js" % "sbt-scalajs" % "1.22.0")
// For the export of sbt-buildinfo's object (BuildInfoGenerator), reached only in a build that has it.
addSbtPlugin("com.eed3si9n" % "sbt-buildinfo" % "0.13.1" % Provided)
// For the export of sbt-native-packager's Docker stage (DockerStage), reached only in a build that has it.
addSbtPlugin("com.github.sbt" % "sbt-native-packager" % "1.11.7" % Provided)

// The compiler the plugin is released with, the checkout's (Cargo.toml's): the default teqVersion.
def compilerOf(base: File): String =
  """(?m)^version\s*=\s*"([^"]+)"""".r.findFirstMatchIn(IO.read(base / ".." / ".." / "Cargo.toml")).map(_.group(1)).getOrElse(sys.error("no version in Cargo.toml"))

// BuildInfo: the plugin's version; the compiler it is released with; and the commit the plugin is built from,
// which its publish checks against the head (central.py).
Compile / sourceGenerators += Def.task {
  val file = (Compile / sourceManaged).value / "dev" / "teq" / "sbt" / "BuildInfo.scala"
  val commit = scala.util.Try(scala.sys.process.Process(Seq("git", "rev-parse", "HEAD"), baseDirectory.value).!!.trim).getOrElse("")
  IO.write(file, s"package dev.teq.sbt\n\nprivate[sbt] object BuildInfo:\n  val version = \"${version.value}\"\n  val compiler = \"${compilerOf(baseDirectory.value)}\"\n  val commit = \"$commit\"\n")
  Seq(file)
}.taskValue
// The same compiler in the jar's manifest, which a later compiler's release reads from Central (bench/ship-release.sh,
// plugin_default) to record the plugin it selects.
Compile / packageBin / packageOptions += Package.ManifestAttributes("Teq-Compiler" -> compilerOf(baseDirectory.value))

// The launchers teqExportAll writes beside teq.lock (Launchers.scala): teq's tools/launcher/teq and
// teq.cmd, the launcher of the version their marker line names, and every version shipped before
// (tools/launcher/shipped/<version>/), so that an unedited launcher of any is replaced. As
// dev/teq/sbt/launcher/<version>/<name>, line ends \n, with `versions` listing them.
Compile / resourceGenerators += Def.task {
  val launchers = baseDirectory.value / ".." / ".." / "tools" / "launcher"
  val out = (Compile / resourceManaged).value / "dev" / "teq" / "sbt" / "launcher"
  val marker = """(?m)^(?:#|rem) teq launcher (\d+):""".r
  def versionOf(f: File) = marker.findFirstMatchIn(IO.read(f)).map(_.group(1).toInt).getOrElse(sys.error(s"$f has no `teq launcher <version>:` line"))
  val current = versionOf(launchers / "teq")
  if versionOf(launchers / "teq.cmd") != current then sys.error(s"${launchers / "teq.cmd"} is not launcher $current, as ${launchers / "teq"} is")
  val shipped = Option((launchers / "shipped").listFiles).toSeq.flatten.filter(_.isDirectory).map(d => d.getName.toInt -> d)
  val all = (shipped :+ (current -> launchers)).sortBy(_._1)
  IO.delete(out)
  val files = for (version, dir) <- all; name <- Seq("teq", "teq.cmd") yield
    val file = out / version.toString / name
    IO.write(file, IO.read(dir / name).replace("\r\n", "\n"))
    file
  IO.write(out / "versions", all.map(_._1).mkString("", "\n", "\n"))
  files :+ (out / "versions")
}.taskValue

// A local publish comes before every repository for every build of the machine, so it never takes a
// release's version: a branch publishes under one of its own (`set version := "0.1.1-<branch>-SNAPSHOT"`).
publishLocalConfiguration := {
  if !isSnapshot.value then
    throw new MessageOnlyException(s"${version.value} is a release's version, which a local publish would stand in for in every build of the machine: publish locally under a SNAPSHOT of the branch's own")
  publishLocalConfiguration.value
}

// The adapter's tests (src/test): recorded graphs of teq's through the adapter into zinc.
libraryDependencies += "org.scalameta" %% "munit" % "1.3.4" % Test
