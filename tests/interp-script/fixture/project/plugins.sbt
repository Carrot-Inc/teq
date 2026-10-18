// The release of sbt-teq the fixture exports with; TEQ_PLUGIN_VERSION names a plugin published locally under a
// version of its own instead (a branch's `sbt --batch 'set version := "0.1.1-<branch>-SNAPSHOT"; ^publishLocal'`).
addSbtPlugin("build.teq" % "sbt-teq" % sys.env.getOrElse("TEQ_PLUGIN_VERSION", "1.0.0"))
