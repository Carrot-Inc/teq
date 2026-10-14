// The oracle's listener for teq's Scala.js reference projects (bench/reference/scalajs.scala's verb `oracle`), added
// to the build's meta-build by sbt's -addPluginSbtFile, so that the repository is left as it is: this directory's
// OracleEvents.scala, which records every event of the build's test run (the script names the directory).
Compile / unmanagedSourceDirectories += file(sys.props.getOrElse("teq.oracle.events", sys.error("teq.oracle.events names no directory")))
