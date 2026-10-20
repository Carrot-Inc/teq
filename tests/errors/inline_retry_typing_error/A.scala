object A:
  val value: Int = SourceMacro.value
  val bad: String = 1
  inline def fail: Int = scala.compiletime.error("EXPANSION_AFTER_RETRY_ERROR")
  val x: Int = fail
// The file suspends at the macro of the run its typing meets and is typed again in the later run, whose
// typing reports the mismatch: no expansion follows, as dotty's later run passes `Phase.isRunnable`, which
// runs no `Inlining` after a typing error (before, the later round expanded `fail` and reported it too).
// expect: A.scala:3:21: error: type mismatch: found 1, required String
// absent: error: EXPANSION_AFTER_RETRY_ERROR
