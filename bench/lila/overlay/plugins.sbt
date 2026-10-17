// The overlay of lila's build for teq's reference runs (bench/lila/lila.scala), added to lila's meta-build by sbt's
// -addPluginSbtFile, so that lila's repository is left as it is: sbt-teq at the version the script passes (from the
// repository it names, where it published the head's), and this directory's LilaTeq.scala, the generators teq runs in
// the place of sbt's (the script names the directory).
resolvers ++= sys.props.get("teq.plugin.repository").map(r => "teq-reference" at r).toSeq
addSbtPlugin("build.teq" % "sbt-teq" % sys.props.getOrElse("teq.plugin.version", sys.error("teq.plugin.version names no sbt-teq version")))
Compile / unmanagedSourceDirectories += file(sys.props.getOrElse("teq.lila.overlay", sys.error("teq.lila.overlay names no overlay directory")))
