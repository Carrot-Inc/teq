// The test scope as scala-cli has it: Draft.test.scala is the
// test scope's, compiled under `--test` alone with its own options
// (CrossSources.scopedSources), so the main program compiles without it and runs.
object Main { def main(args: Array[String]): Unit = println("main") }
