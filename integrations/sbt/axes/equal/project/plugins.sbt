// The plugin under test, under the version check.sh publishes it at (TEQ_PLUGIN_VERSION), from the repository it
// publishes it into, and the plugins whose blocks the export carries; each line the same under sbt 1 and sbt 2.
resolvers ++= sys.env.get("AXES_PLUGIN_REPOSITORY").map("axes" at _).toSeq
addSbtPlugin("build.teq" % "sbt-teq" % sys.env("TEQ_PLUGIN_VERSION"))
addSbtPlugin("org.scala-js" % "sbt-scalajs" % "1.22.0")
addSbtPlugin("com.eed3si9n" % "sbt-buildinfo" % "0.13.1")
addSbtPlugin("com.github.sbt" % "sbt-native-packager" % "1.11.7")
