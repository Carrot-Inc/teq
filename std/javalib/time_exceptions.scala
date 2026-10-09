// The exceptions of `java.time` (time.scala), under the JDK's names and hierarchy; a file of their
// own, as nio_exceptions.scala is, so that a program that reaches `RuntimeException` enters these
// classes and not the calendar's. Left out with time.scala where the class path holds the package.
package java.time:

  @jvmClass("java/time/DateTimeException")
  class DateTimeException(message: String = null) extends RuntimeException(message)

package java.time.temporal:

  @jvmClass("java/time/temporal/UnsupportedTemporalTypeException")
  class UnsupportedTemporalTypeException(message: String = null) extends java.time.DateTimeException(message)
