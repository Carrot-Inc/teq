// The overlay of Airstream's build for teq's reference runs (bench/airstream/airstream.scala), added to its meta-build
// by sbt's -addPluginSbtFile, so that the repository is left as it is: sbt-teq at the version the script passes (sbt
// 1's module, sbt-teq_2.12_1.0, which the build's sbt resolves, from the repository it names, where it published the
// head's), and this directory's AirstreamTeq.scala, the generators teq runs in the place of sbt's (the script names the
// directory).
resolvers ++= sys.props.get("teq.plugin.repository").map(r => "teq-reference" at r).toSeq
addSbtPlugin("build.teq" % "sbt-teq" % sys.props.getOrElse("teq.plugin.version", sys.error("teq.plugin.version names no sbt-teq version")))
Compile / unmanagedSourceDirectories += file(sys.props.getOrElse("teq.airstream.overlay", sys.error("teq.airstream.overlay names no overlay directory")))
