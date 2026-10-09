// The classes of JUnit's that a Scala.js test framework's jar extends or throws (munit's
// `ComparisonFailException`, its `assume`), which Scala.js's `scalajs-junit-test-runtime` has as
// Scala 2.13 IR alone. JavaScript only: on the JVM they are JUnit's own, from its jar.
package org.junit

/** An assertion that two strings are equal, failed: the message names the part of each that
  * differs, in brackets, with at most 20 characters of the shared context around it, as JUnit
  * words it; a null message is the text "null", as JUnit's `AssertionError` makes it. */
class ComparisonFailure(message: String, expected: String, actual: String) extends AssertionError(String.valueOf(message)):
  def getExpected(): String = expected
  def getActual(): String = actual
  override def getMessage(): String = ComparisonFailure.compact(super.getMessage(), expected, actual)

object ComparisonFailure:
  private val context = 20

  private def format(message: String, expected: String, actual: String): String =
    val lead = if message == null || message.isEmpty then "" else message + " "
    if expected != null && expected == actual then lead + "expected: java.lang.String<" + expected + "> but was: java.lang.String<" + actual + ">"
    else lead + "expected:<" + expected + "> but was:<" + actual + ">"

  private def compact(message: String, expected: String, actual: String): String =
    if expected == null || actual == null || expected == actual then format(message, expected, actual)
    else
      val shortest = Math.min(expected.length, actual.length)
      var prefix = 0
      while prefix < shortest && expected.charAt(prefix) == actual.charAt(prefix) do prefix += 1
      var suffix = 0
      while suffix < shortest - prefix && expected.charAt(expected.length - 1 - suffix) == actual.charAt(actual.length - 1 - suffix) do suffix += 1
      val shared = expected.substring(0, prefix)
      val trailing = expected.substring(expected.length - suffix)
      val before = if shared.length <= context then shared else "..." + shared.substring(shared.length - context)
      val after = if trailing.length <= context then trailing else trailing.substring(0, context) + "..."
      def differing(s: String): String = before + "[" + s.substring(prefix, s.length - suffix) + "]" + after
      format(message, differing(expected), differing(actual))

/** An assumption of a test that does not hold: the test is skipped, not failed. */
class AssumptionViolatedException(message: String) extends RuntimeException(message):
  def this(message: String, cause: Throwable) =
    this(message)
    initCause(cause)
