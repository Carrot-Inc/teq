// The equal-facts fixture of the plugin's two sbt axes (check.sh): what both sbt 1 and sbt 2 resolve alike, so that
// the two exports are the same bytes but for the digest of project/build.properties and the hash over the build
// files. No project with a Test configuration depends on another (the order of such a class path is each sbt's
// own, the order fixture's case), every path is set where the two sbts' defaults differ (the Docker stage's), and
// every setting is the build's, `ThisBuild /` or a project's, which sbt 1 and sbt 2 scope alike.
ThisBuild / scalaVersion := "3.8.4"
ThisBuild / teqVersion := "0.1.8"
ThisBuild / teqReleases := sys.env("AXES_RELEASES")
ThisBuild / teqBuildTool := true
// The plugin under test is a SNAPSHOT published locally.
ThisBuild / teqExportSnapshots := true

lazy val root = project.in(file(".")).aggregate(lib, app, gen, sjs)

// A library with munit's suites, a framework's argument, and a generator teq runs.
lazy val lib = project.settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
  Test / unmanagedSourceDirectories := Seq(baseDirectory.value / "test"),
  libraryDependencies += "org.scalameta" %% "munit" % "1.3.4" % Test,
  Test / testOptions += Tests.Argument(TestFrameworks.MUnit, "-b"),
  Compile / teqGenerators += TeqCommand(Seq("sh", "lib/generate.sh"), inputs = Seq("lib/generate.sh")),
)

// An application with BuildInfo and a Docker stage, its main class declared and the stage's directory set.
lazy val app = project.enablePlugins(BuildInfoPlugin, JavaAppPackaging).settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
  Compile / mainClass := Some("app.Main"),
  buildInfoPackage := "app",
  buildInfoKeys := Seq[BuildInfoKey](name, version, scalaVersion),
  Docker / com.typesafe.sbt.packager.Keys.stagingDirectory := baseDirectory.value / "target" / "docker" / "stage",
  teqRunAliases := Map("hello" -> "app.Main"),
)

// Generators sbt alone runs, a source's and a resource's, which the lock records and teq refuses; the managed
// directory the lock lists among its sources set.
lazy val gen = project.settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
  Compile / sourceManaged := baseDirectory.value / "target" / "src_managed" / "main",
  Compile / sourceGenerators += Def.task {
    val file = (Compile / sourceManaged).value / "gen" / "Generated.scala"
    IO.write(file, "package gen\n\nobject Generated { val n: Int = 1 }\n")
    Seq(file)
  }.taskValue,
  Compile / resourceGenerators += Def.task {
    val file = (Compile / resourceManaged).value / "gen.txt"
    IO.write(file, "gen\n")
    Seq(file)
  }.taskValue,
)

// A Scala.js library with munit's suites and a framework's argument.
lazy val sjs = project.enablePlugins(ScalaJSPlugin).settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
  Test / unmanagedSourceDirectories := Seq(baseDirectory.value / "test"),
  libraryDependencies += "org.scalameta" % "munit_sjs1_3" % "1.3.4" % Test,
  Test / testOptions += Tests.Argument(TestFrameworks.MUnit, "-b"),
)
