// The adapter and the JSON reader of the plugin itself, compiled into this build's definition,
// so that the harness runs the plugin's code without a published plugin.
Compile / unmanagedSources ++= {
  val plugin = baseDirectory.value.getParentFile.getParentFile / "src" / "main" / "scala"
  Seq("ApiGraph", "TeqAnalysis", "TeqCompile").map(n => plugin / "sbt" / "internal" / "teq" / s"$n.scala") ++
    Seq("Json", "ArgsFile").map(n => plugin / "dev" / "teq" / "sbt" / s"$n.scala")
}
