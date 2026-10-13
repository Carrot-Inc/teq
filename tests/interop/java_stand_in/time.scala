// A source stand-in for a JDK class, as an application written for the JVM ships one for
// JavaScript: a Scala class, whose members take Scala's rules as under scalac's Scala.js build
// (dotty's `matchNullaryLoosely` asks where a member comes from, never its package's name).
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
