// The dependency-order fixture of the plugin's two sbt axes (check.sh): facts each sbt resolves its own way, which
// each axis's export carries as its sbt has them. A Test configuration of a project that depends on another has
// both projects' products on its class path, in the order of sbt's ClasspathImpl.interSort (sbt 1 visits the
// configurations of the project before its dependencies, sbt 2 the dependencies of each configuration as it goes);
// the Docker stage's default directory is under each sbt's own target; BuildInfo's sbtVersion is the sbt that runs.
ThisBuild / scalaVersion := "3.8.4"
ThisBuild / teqVersion := "0.1.8"
ThisBuild / teqReleases := sys.env("AXES_RELEASES")
ThisBuild / teqBuildTool := true
// The plugin under test is a SNAPSHOT published locally.
ThisBuild / teqExportSnapshots := true

lazy val root = project.in(file(".")).aggregate(lib, app, sjscore, sjsapp)

lazy val lib = project.settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
)

lazy val app = project.enablePlugins(BuildInfoPlugin, JavaAppPackaging).dependsOn(lib).settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
  Test / unmanagedSourceDirectories := Seq(baseDirectory.value / "test"),
  Compile / mainClass := Some("app.Main"),
  buildInfoPackage := "app",
  buildInfoKeys := Seq[BuildInfoKey](name, version, scalaVersion, sbtVersion),
  libraryDependencies += "org.scalameta" %% "munit" % "1.3.4" % Test,
)

lazy val sjscore = project.enablePlugins(ScalaJSPlugin).settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
)

lazy val sjsapp = project.enablePlugins(ScalaJSPlugin).dependsOn(sjscore).settings(
  Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
  Test / unmanagedSourceDirectories := Seq(baseDirectory.value / "test"),
  libraryDependencies += "org.scalameta" % "munit_sjs1_3" % "1.3.4" % Test,
  Test / testOptions += Tests.Argument(TestFrameworks.MUnit, "-b"),
)
