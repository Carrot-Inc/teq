// A small build whose lock (teq.lock, `sbt teqExportAll`) a Scala CLI script resolves its `//> using dep` lines
// through under `teq interp` (tests/interp-script.sh): `tools` holds the script's sources, with the libraries of a
// ZIO command-line tool; cats-core asked at 2.12.0 is evicted by decline's 2.13.0, which the lock records;
// jline's `jdk8` jar is a classified artifact on the classpath (`//> using dep org.jline:jline:3.27.1,classifier=jdk8`);
// `alpha` and `beta` share a source directory, which a script including it cannot tell the owner of; `gamma` builds
// on `alpha`, whose products its classpath holds, so that a script over gamma's sources runs with alpha's; `delta`
// on `badlib`, and `zeta` on `epsilon`, below.
ThisBuild / scalaVersion := "3.8.4"

lazy val tools = project
  .in(file("tools"))
  .settings(
    Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
    libraryDependencies ++= Seq(
      "dev.zio" %% "zio" % "2.1.26",
      "dev.zio" %% "zio-json" % "0.9.2",
      "dev.zio" %% "zio-interop-cats" % "23.1.0.13",
      "com.monovore" %% "decline" % "2.6.2",
      "com.lihaoyi" %% "os-lib" % "0.11.8",
      "co.fs2" %% "fs2-io" % "3.13.0",
      "com.softwaremill.sttp.client4" %% "zio" % "4.0.26",
      "org.typelevel" %% "cats-core" % "2.12.0",
      ("org.jline" % "jline" % "3.27.1").classifier("jdk8"),
    ),
  )

lazy val alpha = project
  .in(file("alpha"))
  .settings(Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "../shared/src"))

lazy val beta = project
  .in(file("beta"))
  .settings(Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "../shared/src"))

lazy val gamma = project
  .in(file("gamma"))
  .dependsOn(alpha)
  .settings(Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"))

// `delta` builds on `badlib`, one of whose sources asks scala-library at a version the lock does not hold: a script
// over delta's sources runs with badlib's, whose `//> using dep` line the lock refuses as the script's own.
lazy val badlib = project
  .in(file("badlib"))

lazy val delta = project
  .in(file("delta"))
  .dependsOn(badlib)
  .settings(Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"))

// `epsilon` compiles at `-source:3.3`, which the lock records (`flags.source`), and `zeta` builds on it at the
// default version: a script over zeta's sources runs with epsilon's, typed at 3.3 as the build compiled them.
lazy val epsilon = project
  .in(file("epsilon"))
  .settings(
    Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"),
    scalacOptions += "-source:3.3",
  )

lazy val zeta = project
  .in(file("zeta"))
  .dependsOn(epsilon)
  .settings(Compile / unmanagedSourceDirectories := Seq(baseDirectory.value / "src"))
