// The tuple the fixture runs under (tests/scalajs-oracle.scala): sbt-scalajs, then the frameworks in build.sbt.
addSbtPlugin("org.scala-js" % "sbt-scalajs" % sys.props.getOrElse("fixture.scalajs", "1.22.0"))
libraryDependencies += "org.scala-js" %% "scalajs-env-jsdom-nodejs" % "1.1.1"
