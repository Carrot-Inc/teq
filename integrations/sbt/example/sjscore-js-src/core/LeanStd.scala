package core

object LeanStd:
  val buildSbtExists: Boolean = FileMacros.existsAtCompileTime("build.sbt")
  val missingFileExists: Boolean = FileMacros.existsAtCompileTime("no-such-file.txt")
