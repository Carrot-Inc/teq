package scala.compat

// scala-library's `Platform`, deprecated since 2.13 for `System`, by which ScalaTest's Scala.js
// runner still times its suites (`TaskRunner`).
object Platform:
  val EOL: String = System.lineSeparator()
  def currentTime: Long = System.currentTimeMillis()
