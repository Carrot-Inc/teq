addSbtPlugin("org.scala-js" % "sbt-scalajs" % "1.22.0")
// For api's BuildInfo, which teq.lock exports as a buildInfo generator.
addSbtPlugin("com.eed3si9n" % "sbt-buildinfo" % "0.13.1")
// For api's Docker stage, which teq.lock exports as its stage block.
addSbtPlugin("com.github.sbt" % "sbt-native-packager" % "1.11.7")
// The release of sbt-teq the example builds with, from Maven Central: the pin moves to a release once it is
// published and read back (bench/release.sh --pin). TEQ_PLUGIN_VERSION names a plugin published locally under a
// version of its own instead (a branch's `sbt --batch 'set version := "0.1.1-<branch>-SNAPSHOT"; publishLocal'`),
// so that a branch's plugin never stands in for a release other builds resolve; check.sh and check-export.sh read
// the pin the same way.
addSbtPlugin("build.teq" % "sbt-teq" % sys.env.getOrElse("TEQ_PLUGIN_VERSION", "1.0.0"))
