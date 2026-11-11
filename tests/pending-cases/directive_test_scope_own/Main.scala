// The test scope as scala-cli has it: Draft.test.scala is the test scope's, compiled under `--test` alone with its own
// options (CrossSources.scopedSources), so the main program compiles without it and runs. teq has no test scope: it
// keeps the file's options out of the main settings but compiles the file with the main sources.
object Main { def main(args: Array[String]): Unit = println("main") }
