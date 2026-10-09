// A source stand-in for a JDK class, as an application written for the JVM ships one for
// JavaScript: its members take the Java parentheses rule.
package java.time

class Instant(val millis: Long):
  def plusMillis(n: Long): Instant = new Instant(millis + n)
  def toEpochMilli(): Long = millis
  override def toString = s"Instant($millis)"

object Instant:
  def now(): Instant = new Instant(1000L)
  def ofEpochMilli(ms: Long): Instant = new Instant(ms)

class Clock:
  def millis(): Long = 5L
  def zone: String = "UTC"
