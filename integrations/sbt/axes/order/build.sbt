// The dependency-order fixture of the plugin's two sbt axes (check.sh): facts each sbt resolves its own way, which
// each axis's export carries as its sbt has them. A Test configuration of a project that depends on another has
// both projects' products on its class path, in the order of sbt's ClasspathImpl.interSort (sbt 1 visits the
// configurations of the project before its dependencies, sbt 2 the dependencies of each configuration as it goes);
// the Docker stage's default directory is under each sbt's own target; BuildInfo's sbtVersion is the sbt that runs;
// the forked test worker runs lib's suites on a pool of the processors under sbt 2 (`testForkedParallel` true by its
// default, `parallelism: auto`) and on one thread under sbt 1 (false by its default, `1`), app's on one thread under
// both (`Test / testForkedParallel := false`). Each module has its own source version: lib and sjscore are Scala 2
// sources (`given` and `enum` identifiers, `-source 3.0-migration` in scalac's two forms), app (`-source:3.9`) and
// sjsapp (the default) declare givens beside them, which no single setting accepts.
ThisBuild / scalaVersion := "3.8.4"
ThisBuild / teqVersion := "0.1.8"
ThisBuild / teqReleases := sys.env("AXES_RELEASES")
ThisBuild / teqBuildTool := true
// The plugin under test is a SNAPSHOT published locally.
ThisBuild / teqExportSnapshots := true

lazy val root = project.in(file(".")).aggregate(lib, app, sjscore, sjsapp)

lazy val lib = project.settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
  scalacOptions ++= Seq("-source", "3.0-migration"),
)

lazy val app = project.enablePlugins(BuildInfoPlugin, JavaAppPackaging).dependsOn(lib).settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
  Test / unmanagedSourceDirectories := Seq(baseDirectory.value / "test"),
  Compile / mainClass := Some("app.Main"),
  buildInfoPackage := "app",
  buildInfoKeys := Seq[BuildInfoKey](name, version, scalaVersion, sbtVersion),
  libraryDependencies += "org.scalameta" %% "munit" % "1.3.4" % Test,
  Test / testForkedParallel := false,
  scalacOptions += "-source:3.9",
)

lazy val sjscore = project.enablePlugins(ScalaJSPlugin).settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
  scalacOptions += "-source:3.0-migration",
)

lazy val sjsapp = project.enablePlugins(ScalaJSPlugin).dependsOn(sjscore).settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
  Test / unmanagedSourceDirectories := Seq(baseDirectory.value / "test"),
  libraryDependencies += "org.scalameta" % "munit_sjs1_3" % "1.3.4" % Test,
  Test / testOptions += Tests.Argument(TestFrameworks.MUnit, "-b"),
)
